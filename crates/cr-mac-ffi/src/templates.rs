//! An encounter template of the user databases as the Mac app edits it,
//! in the record's sheet: the editor of `cr_app::records::templates`
//! described as a form, as the Windows app's templates.rs draws it, its
//! pages chosen as a record's are, and in a dialog over them a line's
//! Properties, the pickers its Add buttons open, the numbers of a monster
//! that comes with an item, and NPC Creation's "Add to Template".

use std::sync::{Arc, Mutex};

use cr_app::npc::{NpcForm, NpcInput};
use cr_app::records::templates::{
    COIN_RANGES, COINS, ESCORT_RANGES, Escort, LineEdit, LineInput, MOST, MOST_HIT_DICE,
    MOST_TREASURE, MonsterFor, NpcFor, Picker, PickerInput, Ranges, SECOND_RANGES, TemplateForm,
    TemplateInput, TemplatePage, item_rows, monster_rows, npc_rows, trap_list,
};
use cr_i18n::tr;
use cr_store::npc::NpcRules;
use cr_store::treasure::Rand;
use cr_store::userdb::Filter;
use cr_store::userdb::templates::{self as tpl, Combo};

use crate::encounter::{MOST_SHOWN, more_shown};
use crate::form::{Builder, FormButton, FormDialog, FormNumber, FormRow};
use crate::record::In;

/// The template editor, with what NPC Creation needs.
pub(crate) struct TemplateEditor {
    pub form: TemplateForm,
    /// NPC Creation in template mode, open for this.
    pub npc: Option<(NpcFor, NpcForm)>,
    pub rules: Arc<NpcRules>,
    pub rand: Arc<Mutex<Rand>>,
}

impl TemplateEditor {
    pub fn new(form: TemplateForm, rules: Arc<NpcRules>, rand: Arc<Mutex<Rand>>) -> Self {
        Self {
            form,
            npc: None,
            rules,
            rand,
        }
    }

    pub fn help(&self) -> u32 {
        if self.npc.is_some() {
            cr_store::help::topic::NPC_CREATION
        } else {
            self.form.help()
        }
    }

    pub fn can_save(&self) -> bool {
        self.npc.is_none() && self.form.can_save()
    }

    /// The pages, by name.
    pub fn tabs() -> Vec<String> {
        TemplatePage::ALL
            .iter()
            .map(|p| p.name().to_owned())
            .collect()
    }

    pub fn tab(&self) -> u32 {
        TemplatePage::ALL
            .iter()
            .position(|p| *p == self.form.page)
            .unwrap_or(0) as u32
    }

    /// NPC Creation's changes, Add to Template (true) and Cancel.
    pub fn npc_input(&mut self, input: NpcInput) {
        if let Some((_, npc)) = &mut self.npc {
            let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
            npc.input(input, &mut rand);
        }
    }

    pub fn npc_done(&mut self, add: bool) {
        let Some((purpose, mut npc)) = self.npc.take() else {
            return;
        };
        if !add {
            return;
        }
        let made = {
            let mut rand = self.rand.lock().unwrap_or_else(|e| e.into_inner());
            npc.generate(&mut rand)
        };
        match made {
            Ok(made) => self
                .form
                .npc_made(purpose, cr_app::records::templates::template_npc(&made)),
            Err(why) => {
                npc.error = Some(why);
                self.npc = Some((purpose, npc));
            }
        }
    }

    /// NPC Creation for `purpose`.
    pub fn open_npc(&mut self, purpose: NpcFor) {
        self.npc = Some((purpose, NpcForm::new(self.rules.clone(), true)));
    }
}

fn input(input: TemplateInput) -> In {
    In::Template(input)
}

fn line(change: LineInput) -> In {
    input(TemplateInput::Line(change))
}

fn pick(change: PickerInput) -> In {
    input(TemplateInput::Picker(change))
}

/// The page in view, and the dialog over it: the escort's numbers, a
/// picker, NPC Creation or a line's Properties, the one opened last.
pub(crate) fn describe(form: &mut Builder<In>, e: &TemplateEditor) -> Option<FormDialog> {
    let f = &e.form;
    match f.page {
        TemplatePage::Template => template_page(form, f),
        TemplatePage::Monsters => {
            let rows = monster_rows(&f.template)
                .into_iter()
                .map(|(text, i)| (text.trim_start().to_owned(), String::new(), i))
                .collect();
            list_page(
                form,
                tr("Monsters associated with this Encounter Template"),
                (tr("Add Monster"), TemplateInput::AddMonster),
                rows,
                (tr("Delete Monster"), TemplateInput::RemoveMonster),
                Some(TemplateInput::OpenMonster),
            );
        }
        TemplatePage::Npcs => {
            let rows = npc_rows(&f.template)
                .into_iter()
                .enumerate()
                .map(|(i, text)| (text.trim_start().to_owned(), String::new(), i))
                .collect();
            list_page(
                form,
                tr("NPCs associated with this Encounter Template"),
                (tr("Add NPC"), TemplateInput::AddNpc),
                rows,
                (tr("Delete NPC"), TemplateInput::RemoveNpc),
                Some(TemplateInput::OpenNpc),
            );
        }
        TemplatePage::Items => {
            let rows = item_rows(&f.template)
                .into_iter()
                .enumerate()
                .map(|(i, (name, under))| (name, under, i))
                .collect();
            list_page(
                form,
                tr("Items associated with this Encounter Template"),
                (tr("Add Item"), TemplateInput::AddItem),
                rows,
                (tr("Delete Item"), TemplateInput::RemoveItem),
                Some(TemplateInput::OpenItem),
            );
        }
        TemplatePage::Traps => {
            let rows = trap_list(&f.template)
                .into_iter()
                .enumerate()
                .map(|(i, trap)| (trap.name, trap.damage.trim().to_owned(), i))
                .collect();
            list_page(
                form,
                tr("Traps"),
                (tr("Add Trap"), TemplateInput::AddTrap),
                rows,
                (tr("Remove Trap"), TemplateInput::RemoveTrap),
                None,
            );
        }
        TemplatePage::Treasure => {
            form.group(
                tr("Treasure Types"),
                tr("How many of each treasure type the lair holds"),
            );
            for (i, (letter, n)) in f.treasure().into_iter().enumerate() {
                form.number(
                    (&letter.to_string(), ""),
                    n.into(),
                    (0, MOST_TREASURE.into()),
                    Vec::new(),
                    move |v| input(TemplateInput::Treasure(i, v as i32)),
                );
            }
        }
        TemplatePage::Table => table_page(form, f),
    }
    let page = form.take_groups();
    let dialog = if let Some(escort) = &f.escort {
        Some(escort_dialog(form, escort))
    } else if let Some(picker) = &f.picker {
        Some(picker_dialog(form, f, picker))
    } else if let Some((_, npc)) = &e.npc {
        Some(npc_dialog(form, npc))
    } else {
        f.line.as_ref().map(|edit| line_dialog(form, edit))
    };
    form.put_groups(page);
    dialog
}

fn template_page(form: &mut Builder<In>, f: &TemplateForm) {
    let (types, chosen) = f.type_choices();
    form.group("", "")
        .text(
            tr("Encounter Template Name"),
            &f.template.name,
            false,
            Vec::new(),
            |name| input(TemplateInput::Name(name)),
        )
        .choice(
            (tr("Monster Type"), ""),
            types,
            Some(chosen),
            Vec::new(),
            |i| input(TemplateInput::MonsterType(i)),
        );
    form.group(tr("Additional Notes for this Encounter"), "")
        .text("", &f.template.notes, true, Vec::new(), |notes| {
            input(TemplateInput::Notes(notes))
        });
}

/// A page of lines: each with Edit… when it has Properties and its
/// Remove, and Add beside the heading.
fn list_page(
    form: &mut Builder<In>,
    title: &str,
    (add, added): (&str, TemplateInput),
    rows: Vec<(String, String, usize)>,
    (remove, removed): (&str, fn(usize) -> TemplateInput),
    open: Option<fn(usize) -> TemplateInput>,
) {
    form.group(title, "");
    let add = form.button(add, "plus", true, input(added));
    form.heading_button(add);
    if rows.is_empty() {
        form.none();
    }
    for (title, subtitle, i) in rows {
        let mut buttons = Vec::new();
        if let Some(open) = open {
            buttons.push(form.button(tr("_Edit…"), "pencil", true, input(open(i))));
        }
        buttons.push(form.button(remove, "minus.circle", true, input(removed(i))));
        form.row(FormRow::Tree {
            title,
            subtitle,
            depth: 0,
            dimmed: false,
            buttons,
        });
    }
}

fn table_page(form: &mut Builder<In>, f: &TemplateForm) {
    let t = &f.template;
    form.group(
        tr("Select the category under which this Encounter Template will appear"),
        "",
    );
    let placements: Vec<(String, String)> = tpl::PLACEMENT_NAMES
        .iter()
        .map(|n| (tr(n).to_owned(), String::new()))
        .collect();
    form.radio(
        &placements,
        Some(tpl::placement_choice(&t.placement)),
        |i| input(TemplateInput::Placement(i)),
    )
    .choice(
        (tr("Frequency"), ""),
        tpl::FREQUENCY_NAMES
            .iter()
            .map(|n| tr(n).to_owned())
            .collect(),
        Some(tpl::frequency_choice(t.frequency)),
        Vec::new(),
        |i| input(TemplateInput::Frequency(i)),
    )
    .number(
        (tr("Number Of Hit Dice"), ""),
        t.hit_dice.into(),
        (0, MOST_HIT_DICE.into()),
        Vec::new(),
        |v| input(TemplateInput::HitDice(v as i32)),
    );
    form.group(tr("Climate and Terrain"), "");
    form.text(
        tr("Category"),
        &t.climate_terrain,
        false,
        Vec::new(),
        |text| input(TemplateInput::Category(text)),
    );
    let terrains = f.data().terrains.clone();
    if !terrains.is_empty() {
        let names = terrains.clone();
        form.choice(
            (&tr!("Choose {field}", field = tr("Category")), ""),
            terrains,
            None,
            Vec::new(),
            move |i| input(TemplateInput::Category(names[i].clone())),
        );
    }
    form.group(
        tr("Climate and Terrain"),
        tr("Chosen one by one in the ~Custom category"),
    );
    let custom = f.custom_terrain();
    let places: Vec<(String, String, bool, bool)> = f
        .climates()
        .into_iter()
        .map(|(place, on)| (place.to_owned(), String::new(), on, custom))
        .collect();
    form.checks(&places, |i, on| input(TemplateInput::Climate(i, on)));
}

/// "N d S * M + B", each kept in its range.
fn dice(
    form: &mut Builder<In>,
    title: &str,
    values: [i32; 4],
    ranges: Ranges,
    changed: impl Fn(usize, i32) -> In + Clone + Send + Sync + 'static,
) {
    let parts = [tr("Dice"), tr("Sides"), tr("Multiplier"), tr("Bonus")];
    let numbers: Vec<FormNumber> = (0..4)
        .map(|k| {
            let changed = changed.clone();
            let (min, max) = ranges[k];
            FormNumber {
                id: form.on_number((min.into(), max.into()), move |v| changed(k, v as i32)),
                label: format!("{title}: {}", parts[k]),
                value: values[k].into(),
                min: min.into(),
                max: max.into(),
            }
        })
        .collect();
    form.row(FormRow::Numbers {
        title: title.to_owned(),
        numbers,
        separators: vec!["d".to_owned(), "*".to_owned(), "+".to_owned()],
        enabled: true,
    });
}

/// A list of a line's Properties, each entry with Remove when it may be
/// removed, and Add beside the heading.
fn entries(
    form: &mut Builder<In>,
    (title, caption): (&str, &str),
    rows: Vec<(String, String, Option<In>)>,
    add: (&str, In),
) {
    form.group(title, caption);
    let add = form.button(add.0, "plus", true, add.1);
    form.heading_button(add);
    if rows.is_empty() {
        form.none();
    }
    for (title, subtitle, remove) in rows {
        let buttons: Vec<FormButton> = remove
            .map(|remove| form.button(tr("Remove"), "minus.circle", true, remove))
            .into_iter()
            .collect();
        form.row(FormRow::Tree {
            title,
            subtitle,
            depth: 0,
            dimmed: false,
            buttons,
        });
    }
}

/// A line's Properties.
fn line_dialog(form: &mut Builder<In>, edit: &LineEdit) -> FormDialog {
    let (first, percent) = edit.first();
    let item = edit.item();
    form.group(item.map_or("", |p| p.part.head.name.as_str()), "");
    let first_title = if item.is_some() {
        tr("Number of Items")
    } else {
        tr("Number Appearing")
    };
    dice(form, first_title, first, edit.first_ranges(), |k, v| {
        line(LineInput::Dice(k, v))
    });
    form.number(
        (
            tr("Percent Chance"),
            tr("There is a percent chance that these appear"),
        ),
        percent.into(),
        (1, 100),
        Vec::new(),
        |v| line(LineInput::Percent(v as i32)),
    );
    let (dependent, per, beyond) = edit.dependent();
    form.toggle((tr("Dependent on the following"), ""), dependent, |on| {
        line(LineInput::Dependent(on))
    });
    if dependent {
        form.number(
            (tr("For every"), tr("Enter 0 for a one time addition")),
            per.into(),
            (0, MOST.into()),
            Vec::new(),
            |v| line(LineInput::Per(v as i32)),
        )
        .number(
            (tr("Beyond"), ""),
            beyond.into(),
            (0, MOST.into()),
            Vec::new(),
            |v| line(LineInput::Beyond(v as i32)),
        );
        let choose = form.button(tr("_Choose…"), "", true, line(LineInput::ChooseReference));
        form.button_row(tr("Monster or Monster Type"), &edit.reference(), choose);
    }
    match edit {
        LineEdit::Item { .. } => item_extra(form, edit),
        _ => random_and_second(form, edit),
    }
    let close = form.button(
        tr("_Cancel"),
        "",
        true,
        input(TemplateInput::CloseLine(false)),
    );
    let accept = form.button(tr("_OK"), "", true, input(TemplateInput::CloseLine(true)));
    FormDialog {
        title: edit.title().to_owned(),
        groups: form.take_groups(),
        close,
        accept: Some(accept),
    }
}

/// A monster or NPC line's random list, and its combo with the second
/// box it shows.
fn random_and_second(form: &mut Builder<In>, edit: &LineEdit) {
    let npc = matches!(edit, LineEdit::Npc { .. });
    let rows = edit
        .random()
        .into_iter()
        .enumerate()
        .map(|(k, (title, under, removable))| {
            (
                title,
                under,
                removable.then(|| line(LineInput::RemoveRandom(k))),
            )
        })
        .collect();
    let caption = if npc {
        tr("Each appearance picks one of these NPCs")
    } else {
        tr("Each appearance picks one of these monsters")
    };
    let add = if npc {
        tr("Add NPC")
    } else {
        tr("Add Monster")
    };
    entries(
        form,
        (tr("Randomly selected from the following list"), caption),
        rows,
        (add, line(LineInput::AddRandom)),
    );
    let combo = edit.combo().unwrap_or(Combo::Appear);
    form.group("", "").choice(
        (tr("These then"), ""),
        Combo::NAMES.iter().map(|n| tr(n).to_owned()).collect(),
        Some(combo.index() as usize),
        Vec::new(),
        |i| line(LineInput::Combo(i)),
    );
    if combo != Combo::Appear {
        dice(
            form,
            tr("Number Appearing"),
            edit.second_dice(),
            SECOND_RANGES,
            |k, v| line(LineInput::SecondDice(k, v)),
        );
        let (who, choose) = if npc {
            (tr("NPC"), tr("_NPC…"))
        } else {
            (tr("Monster"), tr("_Monster…"))
        };
        let choose = form.button(choose, "", true, line(LineInput::ChooseSecond));
        form.button_row(who, &edit.second(), choose);
    }
}

/// An item's coins and the monsters that come with it.
fn item_extra(form: &mut Builder<In>, edit: &LineEdit) {
    let (coin, coin_dice) = edit.coins();
    form.group(tr("Coins"), tr("Coins each item holds"));
    dice(form, tr("Containing"), coin_dice, COIN_RANGES, |k, v| {
        line(LineInput::CoinDice(k, v))
    });
    form.choice(
        (tr("Coins"), ""),
        COINS.iter().map(|c| tr(c).to_owned()).collect(),
        Some(coin),
        Vec::new(),
        |i| line(LineInput::Coin(i)),
    );
    for per_item in [false, true] {
        let title = if per_item {
            tr("For each Item...")
        } else {
            tr("These Monsters will appear if the previous Item(s) appear...")
        };
        let rows = edit
            .escorts(per_item)
            .into_iter()
            .enumerate()
            .map(|(k, text)| {
                (
                    text,
                    String::new(),
                    Some(line(LineInput::RemoveEscort(per_item, k))),
                )
            })
            .collect();
        entries(
            form,
            (title, ""),
            rows,
            (tr("Add Monster"), line(LineInput::AddEscort(per_item))),
        );
    }
    let rows = edit
        .doubles()
        .iter()
        .enumerate()
        .map(|(k, name)| {
            (
                name.clone(),
                String::new(),
                Some(line(LineInput::RemoveDouble(k))),
            )
        })
        .collect();
    entries(
        form,
        (tr("Double the number of these Monsters..."), ""),
        rows,
        (tr("Add Monster"), line(LineInput::AddDouble)),
    );
}

/// A picker: its filters, the search and the list, each entry with the
/// button that takes it.
fn picker_dialog(form: &mut Builder<In>, f: &TemplateForm, picker: &Picker) -> FormDialog {
    form.group("", "");
    match picker {
        Picker::Monster(p) => {
            let origin = match p.origin {
                Filter::All => 0,
                Filter::Standard => 1,
                Filter::Custom => 2,
            };
            form.choice(
                (tr("Show Standard or Custom Records"), ""),
                Filter::NAMES.iter().map(|n| tr(n).to_owned()).collect(),
                Some(origin),
                Vec::new(),
                |i| pick(PickerInput::Origin(i)),
            );
            if p.purpose == MonsterFor::Reference {
                form.toggle(
                    (tr("Select Type instead of Monster"), ""),
                    p.instead,
                    |on| pick(PickerInput::Instead(on)),
                );
            }
            if !p.instead {
                form.choice(
                    (tr("Monster Types"), ""),
                    f.picker_types(),
                    Some(p.kind),
                    Vec::new(),
                    |i| pick(PickerInput::Kind(i)),
                );
            }
        }
        Picker::Item(p) => {
            let categories: Vec<String> = f
                .data()
                .categories
                .iter()
                .map(|(_, name)| tr(name).to_owned())
                .collect();
            form.choice(
                (tr("Part Type"), ""),
                categories,
                Some(p.category),
                Vec::new(),
                |i| pick(PickerInput::Category(i)),
            );
        }
        Picker::Trap(_) => {}
    }
    let search = match picker {
        Picker::Monster(p) => &p.search,
        Picker::Item(p) => &p.search,
        Picker::Trap(p) => &p.search,
    };
    form.text(tr("Search"), search, false, Vec::new(), |text| {
        pick(PickerInput::Search(text))
    });
    let adds = !matches!(
        picker,
        Picker::Monster(p) if matches!(p.purpose, MonsterFor::Second | MonsterFor::Reference)
    );
    let take = if adds { tr("_Add") } else { tr("_Choose…") };
    let rows = f.picker_rows();
    form.group(picker.title(), "");
    if rows.is_empty() {
        form.row(FormRow::Note {
            text: tr("No Results").to_owned(),
            secondary: true,
        });
    }
    for r in rows.iter().take(MOST_SHOWN) {
        let subtitle = match (r.custom, r.subtitle.is_empty()) {
            (false, _) => r.subtitle.clone(),
            (true, true) => tr("Custom").to_owned(),
            (true, false) => format!("{} · {}", tr("Custom"), r.subtitle),
        };
        let button = form.button(take, "", true, In::Pick(r.index));
        form.row(FormRow::Tree {
            title: r.title.clone(),
            subtitle,
            depth: 0,
            dimmed: false,
            buttons: vec![button],
        });
    }
    more_shown(form, rows.len());
    let close = form.button(
        tr("_Cancel"),
        "",
        true,
        input(TemplateInput::ClosePicker(false)),
    );
    FormDialog {
        title: picker.title().to_owned(),
        groups: form.take_groups(),
        close,
        accept: None,
    }
}

/// How many of the monster picked come with the item.
fn escort_dialog(form: &mut Builder<In>, escort: &Escort) -> FormDialog {
    form.group("", "")
        .value(tr("Monster"), "", &escort.monster.name);
    dice(
        form,
        tr("Number Appearing"),
        escort.dice,
        ESCORT_RANGES,
        |k, v| input(TemplateInput::EscortDice(k, v)),
    );
    let close = form.button(
        tr("_Cancel"),
        "",
        true,
        input(TemplateInput::CloseEscort(false)),
    );
    let accept = form.button(
        tr("_Add"),
        "",
        true,
        input(TemplateInput::CloseEscort(true)),
    );
    FormDialog {
        title: tr("Select a Monster and the number appearing").to_owned(),
        groups: form.take_groups(),
        close,
        accept: Some(accept),
    }
}

/// NPC Creation in template mode: Add to Template hands the NPC to the
/// template instead of saving it.
fn npc_dialog(form: &mut Builder<In>, npc: &NpcForm) -> FormDialog {
    crate::npc::npc(form, npc, In::Npc);
    let close = form.button(tr("_Cancel"), "", true, In::NpcDone(false));
    let accept = form.button(tr("Add to _Template"), "", true, In::NpcDone(true));
    FormDialog {
        title: tr("New NPC").to_owned(),
        groups: form.take_groups(),
        close,
        accept: Some(accept),
    }
}
