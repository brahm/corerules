# Built on Fedora 44 so the package installs there and on newer releases.
# Cargo fetches the crates while building, so the build needs network
# access (plain rpmbuild, not mock or Koji).
%global debug_package %{nil}

Name:           corerules
Version:        %{?version_override}%{!?version_override:0.9.0}
Release:        1%{?dist}
Summary:        Browse and edit AD&D Core Rules 2.0 characters and rules
License:        MIT
URL:            https://github.com/brahm/corerules
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  cargo >= 1.88
BuildRequires:  rust >= 1.88
BuildRequires:  gcc
BuildRequires:  pkgconfig(gtk4) >= 4.14
BuildRequires:  pkgconfig(libadwaita-1) >= 1.5
BuildRequires:  desktop-file-utils
BuildRequires:  libappstream-glib

Requires:       hicolor-icon-theme

%description
A modern GTK replica of AD&D Core Rules 2.0, the 1998 Windows character
manager for Advanced Dungeons & Dragons 2nd Edition. It reads and writes the
files of the original program and needs a Core Rules installation or CD.

%prep
%autosetup

%build
cargo build --release --locked

%install
packaging/install.sh %{buildroot} %{_prefix} target/release
rm -r %{buildroot}%{_datadir}/licenses

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/io.github.brahm.CoreRules.desktop
appstream-util validate-relax --nonet %{buildroot}%{_metainfodir}/io.github.brahm.CoreRules.metainfo.xml

%files
%license LICENSE
%doc README.md
%{_bindir}/corerules
%{_bindir}/cr
%{_datadir}/applications/io.github.brahm.CoreRules.desktop
%{_datadir}/icons/hicolor/*/apps/io.github.brahm.CoreRules.png
%{_metainfodir}/io.github.brahm.CoreRules.metainfo.xml

%changelog
* Fri Oct 23 2026 Brahm - 0.9.0-1
- Beta: Update Character, Enter Character, kits, the remaining part and
  template editors, Help on every screen and the original's remaining
  dialogs

* Mon Oct 19 2026 Brahm - 0.8.1-1
- Beta: monster types, monsters, races and characters show their pictures;
  fixes for the encounter NPC text and the translated metainfo

* Mon Oct 19 2026 Brahm - 0.8.0-1
- Beta: Extract Data from CD... copies the rules data from the Core Rules
  CD or its ISO image, with no script

* Sun Oct 18 2026 Brahm - 0.7.2-1
- Beta: packages made with the patched Expansion import, and the
  community add-ons come as one package

* Sat Oct 17 2026 Brahm - 0.7.1-1
- Beta: the Weapons section has a sword icon

* Sat Oct 17 2026 Brahm - 0.7.0-1
- Beta: interface in Brazilian Portuguese, Spanish, Italian, French and
  German, with a language choice in Preferences
- Every file format specified in docs/formats

* Sat Oct 17 2026 Brahm - 0.6.0-1
- Party management and character deletion; Core Rules 2.0 installations
  are read-only and imported into the Expansion
- Windows list order, Player's Option NPC details, Table Rolls first roll

* Sat Oct 17 2026 Brahm - 0.5.0-1
- DM Toolkit packages (.xpx, .xpt): Import, Export, New Package and Edit
  Package, matching both original programs byte for byte

* Sat Oct 17 2026 Brahm - 0.4.0-1
- Treasure and encounter generators with saved bundles, encounters and 2-20
  tables
- NPC generator: New NPC and New NPC Group, Core Rules and Player's Option
- Book reader, Table Rolls and editors for every user database
- Run without Wine from a data package made from an installation

* Sun Oct 11 2026 Brahm - 0.3.0-1
- Character advancement: add experience (prime requisite bonus, split
  between classes), level up with hit point rolls, Skills & Powers points
  and racial level limits; spell slots per spell level
- Character sheet: view, print, save as PDF and export HTML or text in the
  original's layout

* Sat Oct 10 2026 Brahm - 0.2.0-1
- Create characters (Core Rules and Player's Option) and edit their
  proficiencies, spells, spellbooks, equipment and money; humans can add a class

* Sat Oct 10 2026 Brahm - 0.1.1-1
- New app icon

* Sat Oct 10 2026 Brahm - 0.1.0-1
- First preview release
