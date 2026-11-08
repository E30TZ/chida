Name:           chida
Version:        1.0.0
Release:        1%{?dist}
Summary:        Visual full-stack builder written in Rust
License:        MIT
URL:            https://github.com/E30TZ/chida
Source0:        %{name}-%{version}.tar.gz
BuildRequires:  rust >= 1.85
BuildRequires:  cargo
ExclusiveArch:  %{rust_arches}

%description
Chida is a visual full-stack builder written in Rust. Design pages, data
tables and logic flows in one project, then export a render of the design
or a complete Rust application: an Axum backend, a Leptos frontend, the
SQL schema and an OpenAPI description.

This package ships the chida command line tool and the demo outputs.

%prep
%setup -q -n %{name}-%{version}/src

%build
cargo build --release --offline || cargo build --release

%install
install -Dm755 target/release/chida %{buildroot}%{_bindir}/chida
install -Dm644 packaging/chida.desktop %{buildroot}%{_datadir}/applications/chida.desktop
install -Dm644 apps/desktop/src-tauri/icons/icon.png %{buildroot}%{_datadir}/icons/hicolor/512x512/apps/chida.png
install -Dm644 LICENSE %{buildroot}%{_datadir}/licenses/%{name}/LICENSE
install -Dm644 README.md %{buildroot}%{_docdir}/%{name}/README.md

%files
%license LICENSE
%doc README.md
%{_bindir}/chida
%{_datadir}/applications/chida.desktop
%{_datadir}/icons/hicolor/512x512/apps/chida.png

%changelog
* Sat Nov 08 2026 Chida contributors - 1.0.0-1
- Foundation release: core model, renderer, data and logic crates,
  full-stack code generation and the chida CLI.
