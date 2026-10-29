//! Help: the installation's WinHelp files on a page of the window. Beside
//! the topic, the contents file's entries as a tree, the file's titled
//! topics, or the topics a search finds; the topic itself in a WebView2,
//! as WinHelp draws it, with its links and popups.

use std::sync::Arc;

use cr_app::help::{self as shared, Files, Followed, Place, Row, Target};
use cr_formats::winhelp::Link;
use cr_i18n::tr;
use cr_win::html;
use cr_win::text::plain;
use windows_reactor::*;

use super::{Message, Shell};
use crate::ui;

/// What the page posts once it has loaded when it is to be printed.
pub const PRINT: &str = "print";

/// The help files the book selector opens, with their names there.
fn books() -> [(&'static str, String); 3] {
    [
        (shared::PROGRAM, plain(tr("_Help"))),
        (shared::MANUAL, plain(tr("_Online Manual"))),
        (shared::TOME, plain(tr("_Tome of Help"))),
    ]
}

/// The help files a book selector entry opens.
pub fn book(index: usize) -> Option<&'static str> {
    books().get(index).map(|(file, _)| *file)
}

/// A topic, ready to show.
#[derive(Clone)]
pub struct Shown {
    pub place: Place,
    pub title: String,
    /// The file's title, when it is not the topic's.
    pub file_title: String,
    /// The file's title.
    pub book_title: String,
    pub page: html::Page,
    /// The rows beside it, when they change with it.
    pub rows: Option<Arc<Vec<Row>>>,
    /// Its row among them.
    pub row: Option<usize>,
    /// Topics Back returns to, after it is shown.
    pub remember: bool,
}

#[derive(Clone, Default)]
pub struct HelpPage {
    /// The file the rows list, lower case.
    pub listed: String,
    pub query: String,
    pub rows: Arc<Vec<Row>>,
    pub shown: Option<Arc<Shown>>,
    /// Counts the topics shown, for the WebView2 to show each once.
    pub generation: u64,
    /// Topics Back returns to.
    pub history: Vec<Place>,
    /// The file of the popup shown and its links.
    pub popup: Option<(String, Vec<Link>)>,
    /// Print the next topic once it shows: the Monsters screen's Print.
    pub print: bool,
    /// The topic shown is printed once it shows.
    pub printing: bool,
    /// The row of the topic shown, whose branch of the tree is open.
    pub row: Option<usize>,
    /// The title of the file listed, for the book selector when it is
    /// none of its books.
    pub listed_title: String,
    /// In a narrow window, the list shows in place of the topic.
    pub side_open: bool,
}

/// The side's width.
const SIDE: f64 = 300.0;

/// The width below which the side and the topic take turns.
const NARROW: f64 = 760.0;

/// Whether the window is too narrow for the side beside the topic.
pub fn narrow() -> bool {
    ui::room() < NARROW
}

/// A row's text in the tree: its title on one line, cut with an ellipsis
/// where it would run past the side, which a tree item does not do itself.
fn row_text(row: &Row) -> String {
    // About 7.5 px a character, less the tree's indent for the level.
    let indent = 24.0 * f64::from(row.level.saturating_sub(1));
    let most = ((SIDE - 56.0 - indent) / 7.5).max(12.0) as usize;
    let title = row.title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.chars().count() <= most {
        return title;
    }
    let cut: String = title.chars().take(most - 1).collect();
    format!("{}…", cut.trim_end())
}

/// The row a tree item names by its text.
pub fn row_named<'a>(rows: &'a [Row], text: &str) -> Option<&'a Row> {
    rows.iter().find(|r| r.is_topic() && row_text(r) == text)
}

/// Shows topic `place` with what goes beside it: the rows of its file
/// when `listed` (the file listed now) neither is it nor lists it.
pub fn show(
    files: &Files,
    place: Place,
    listed: &str,
    rows: &[Row],
    remember: bool,
) -> Result<Box<Shown>, String> {
    let missing = || tr("That topic is not in this installation").to_owned();
    let help = files.file(&place.file).ok_or_else(missing)?;
    let topic = help.topics.get(place.topic).ok_or_else(missing)?;
    let title = shared::title(&help, topic);
    let book_title = help.title.clone();
    let file_title = if help.title == title {
        String::new()
    } else {
        help.title.clone()
    };
    let new_rows = (listed != place.file && !shared::lists(rows, &place))
        .then(|| Arc::new(files.rows(&place.file, "")));
    let row = files.row_of(new_rows.as_deref().map_or(rows, |r| r.as_slice()), &place);
    Ok(Box::new(Shown {
        page: html::topic(&help, topic),
        title,
        file_title,
        book_title,
        rows: new_rows,
        row,
        remember,
        place,
    }))
}

/// Opens `file` at `target`.
pub fn open(
    files: &Files,
    file: &str,
    target: &Target,
    listed: &str,
    rows: &[Row],
) -> Result<Box<Shown>, String> {
    let place = files
        .resolve(file, target)
        .ok_or_else(|| tr!("The help topic is not in {file}", file = file.to_owned()))?;
    show(files, place, listed, rows, true)
}

/// What a link of the topic or of its popup leads to, ready to show.
#[derive(Clone)]
pub enum Opened {
    Shown(Box<Shown>),
    Popup(String, html::Page),
    Web(String),
    Missing,
}

pub fn follow(files: &Files, from: &str, link: &Link, listed: &str, rows: &[Row]) -> Opened {
    match files.follow(from, link) {
        Followed::Go(place) => match show(files, place, listed, rows, true) {
            Ok(shown) => Opened::Shown(shown),
            Err(_) => Opened::Missing,
        },
        Followed::Popup(place) => {
            let page = files.file(&place.file).and_then(|help| {
                let topic = help.topics.get(place.topic)?;
                Some(html::popup(&help, topic))
            });
            match page {
                Some(page) => Opened::Popup(place.file, page),
                None => Opened::Missing,
            }
        }
        Followed::Web(uri) => Opened::Web(uri),
        Followed::Missing => Opened::Missing,
    }
}

/// The rows as a tree, the contents file's headings over their entries;
/// the branch of the topic shown is open.
fn tree(rows: &[Row], current: Option<usize>) -> Vec<TreeNode> {
    // The rows each row is under, by index.
    let mut parents: Vec<Option<usize>> = Vec::with_capacity(rows.len());
    let mut stack: Vec<usize> = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        while stack
            .last()
            .is_some_and(|&top| rows[top].level >= row.level)
        {
            stack.pop();
        }
        parents.push(stack.last().copied());
        stack.push(i);
    }
    let mut open = vec![false; rows.len()];
    let mut at = current.and_then(|i| parents[i]);
    while let Some(i) = at {
        open[i] = true;
        at = parents[i];
    }
    fn node(i: usize, rows: &[Row], parents: &[Option<usize>], open: &[bool]) -> TreeNode {
        let children = (i + 1..rows.len())
            .filter(|&c| parents[c] == Some(i))
            .map(|c| node(c, rows, parents, open));
        TreeNode::new(i.to_string(), row_text(&rows[i]))
            .expanded(open[i])
            .children(children)
    }
    (0..rows.len())
        .filter(|&i| parents[i].is_none())
        .map(|i| node(i, rows, &parents, &open))
        .collect()
}

pub fn view(shell: &Shell, page: &HelpPage, context: &mut ViewContext<Shell>) -> View {
    let mut books: Vec<String> = books().into_iter().map(|(_, name)| name).collect();
    // Another file listed (a book's, a search's or a link's) shows as an
    // entry of its own rather than none.
    let book = help_books_position(&page.listed).or_else(|| {
        (!page.listed_title.is_empty()).then(|| {
            books.push(page.listed_title.clone());
            books.len() - 1
        })
    });
    let narrow = narrow();
    let side = Grid::new()
        .rows([GridLength::Auto, GridLength::Auto, GridLength::STAR])
        .row_spacing(8.0)
        .margin(if narrow {
            Thickness::new(16.0, 0.0, 16.0, 12.0)
        } else {
            Thickness::new(24.0, 24.0, 8.0, 12.0)
        })
        .children((
            ComboBox::new()
                .automation_name(plain(tr("_Help")))
                .items_source(books)
                .selected_index(book)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .on_selection_changed(context.callback(Message::HelpBook)),
            AutoSuggestBox::new()
                .grid_row(1)
                .text(page.query.clone())
                .placeholder_text(tr("Search the topics"))
                .automation_name(tr("Search the topics"))
                .on_text_changed(context.callback(Message::HelpSearch)),
            Grid::new().grid_row(2).keyed_children([KeyedView::new(
                // A new tree for each list: its rows are not edited.
                format!("{}|{}", page.listed, page.query),
                TreeView::new()
                    .automation_name(tr("Contents"))
                    .on_item_invoked(context.callback(Message::HelpRow))
                    .nodes(tree(&page.rows, page.row)),
            )]),
        ));

    let (title, file_title) = match &page.shown {
        Some(shown) => (shown.title.clone(), shown.file_title.clone()),
        None => (plain(tr("_Help")), String::new()),
    };
    let mut commands = Vec::new();
    if narrow {
        commands.push(KeyedView::new(
            "contents",
            ui::command(
                tr("Contents"),
                Symbol::List,
                true,
                context.message(Message::HelpSide(!page.side_open)),
            ),
        ));
    }
    commands.extend([
        KeyedView::new(
            "back",
            ui::command(
                tr("Back"),
                Symbol::Back,
                !page.history.is_empty(),
                context.message(Message::HelpBack),
            ),
        ),
        KeyedView::new(
            "print",
            ui::command(
                &plain(tr("_Print…")),
                Symbol::Print,
                page.shown.is_some(),
                context.message(Message::HelpPrint),
            ),
        ),
    ]);
    let header = Grid::new()
        .columns([GridLength::STAR, GridLength::Auto])
        .margin(if narrow {
            Thickness::new(16.0, 24.0, 16.0, 8.0)
        } else {
            Thickness::new(16.0, 24.0, 24.0, 8.0)
        })
        .children((
            StackPanel::new()
                .spacing(2.0)
                .vertical_alignment(VerticalAlignment::Center)
                .children((ui::subtitle(title), ui::caption(file_title))),
            CommandBar::new()
                .grid_column(1)
                .horizontal_alignment(HorizontalAlignment::Right)
                .collection_slot(CommandBarSlot::PrimaryCommands, commands),
        ));

    // The topic shows once the control has its CoreWebView2, and again
    // for each new topic.
    let help_view = shell.help_view.clone();
    context.use_effect("help-view", (), move || {
        Some(Box::new(move || help_view.forget()))
    });
    if let Some(shown) = page.shown.clone() {
        let help_view = shell.help_view.clone();
        let sender = context.sender();
        // The Monsters screen's Print: the page asks for it once loaded.
        let html = if page.printing {
            shown.page.html.replace(
                "</body>",
                &format!(
                    "<script>addEventListener('load',()=>\
                     post('{PRINT}'))</script></body>"
                ),
            )
        } else {
            shown.page.html.clone()
        };
        context.use_effect(
            "help-topic",
            (page.generation, shell.web_tries),
            move || {
                let links = sender.clone();
                help_view.request(
                    sender,
                    move |web| {
                        let links = links.clone();
                        web.on_message(move |link| {
                            links.send(Message::HelpLink(link));
                        });
                    },
                    move |web| web.show(&html),
                );
                None
            },
        );
    }
    let topic = Border::new()
        .margin(if narrow {
            Thickness::new(16.0, 0.0, 16.0, 16.0)
        } else {
            Thickness::new(16.0, 0.0, 24.0, 24.0)
        })
        .corner_radius(8.0)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(1.0)
        .content(
            WebView2::new()
                .element_ref(&shell.help_view.control)
                .automation_name(title_for_readers(page)),
        );
    // One layout for both widths, only its sizes changing: a WebView2
    // moved to another parent is left blank. In a narrow window the side
    // and the topic take turns under the header, the topic kept (hidden)
    // for its WebView2 not to start again.
    let side_open = narrow && page.side_open;
    Grid::new()
        .columns([
            GridLength::Pixel(if narrow { 0.0 } else { SIDE }),
            GridLength::STAR,
        ])
        .rows([GridLength::Auto, GridLength::STAR])
        .children((
            Border::new()
                .grid_column(if narrow { 1 } else { 0 })
                .grid_row(if narrow { 1 } else { 0 })
                .grid_row_span(if narrow { 1 } else { 2 })
                .height(hidden(narrow && !side_open))
                .content(side),
            Border::new().grid_column(1).content(header),
            Border::new()
                .grid_column(1)
                .grid_row(1)
                .height(hidden(side_open))
                .content(topic),
        ))
}

/// No height when `hide`, which hides a part without taking it away.
fn hidden(hide: bool) -> Option<f64> {
    hide.then_some(0.0)
}

/// Which of the book selector's books `listed` is.
fn help_books_position(listed: &str) -> Option<usize> {
    books()
        .iter()
        .position(|(file, _)| listed.eq_ignore_ascii_case(file))
}

fn title_for_readers(page: &HelpPage) -> String {
    page.shown
        .as_ref()
        .map_or_else(|| plain(tr("_Help")), |s| s.title.clone())
}
