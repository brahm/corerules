# Built on Fedora 44 so the package installs there and on newer releases.
# Cargo fetches the crates while building, so the build needs network
# access (plain rpmbuild, not mock or Koji).
%global debug_package %{nil}

Name:           corerules
Version:        %{?version_override}%{!?version_override:0.1.0}
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
%{_datadir}/icons/hicolor/scalable/apps/io.github.brahm.CoreRules.svg
%{_metainfodir}/io.github.brahm.CoreRules.metainfo.xml

%changelog
* Sat Oct 10 2026 Brahm - 0.1.0-1
- First preview release
