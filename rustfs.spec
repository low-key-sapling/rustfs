%global _enable_debug_packages 0
%global _empty_manifest_terminate_build 0
%global prerelease beta.12
Name:           rustfs
Version:        1.0.0
Release:        beta.12
Summary:       ZfFS S3-compatible distributed object storage

License:        Apache-2.0
URL:            https://github.com/rustfs/rustfs
Source0:        https://github.com/rustfs/rustfs/archive/refs/tags/%{version}-%{prerelease}.tar.gz

BuildRequires: cargo
BuildRequires: rust
BuildRequires: mold
BuildRequires: pango-devel
BuildRequires: cairo-devel
BuildRequires: cairo-gobject-devel
BuildRequires: gdk-pixbuf2-devel
BuildRequires: atk-devel
BuildRequires: gtk3-devel
BuildRequires: libsoup-devel
BuildRequires: cmake
BuildRequires: clang-devel

%description
ZfFS is a high-performance S3-compatible distributed object storage product based on the RustFS compatibility core. It preserves existing storage, protocol, and operational compatibility while providing an independent ZfFS binary and release identity under the Apache License 2.0.

%prep 
%autosetup -n %{name}-%{version}-%{prerelease}

%build
# Set the target directory according to the schema
export CMAKE=$(which cmake3)
%ifarch x86_64 || aarch64 || loongarch64
    TARGET_DIR="target/%_arch"
    PLATFORM=%_arch-unknown-linux-gnu
%else
    TARGET_DIR="target/unknown"
    PLATFORM=unknown-platform
%endif

# Set CARGO_TARGET_DIR and build the project
#CARGO_TARGET_DIR=$TARGET_DIR RUSTFLAGS="-C link-arg=-fuse-ld=mold" cargo build --release --package rustfs
%ifarch loongarch64
CFLAGS="-mcmodel=medium" CARGO_TARGET_DIR=$TARGET_DIR RUSTFLAGS="-C link-arg=-fuse-ld=mold -C link-arg=-lm" cargo build --release --target $PLATFORM -p rustfs --bins
%else
CARGO_TARGET_DIR=$TARGET_DIR RUSTFLAGS="-C link-arg=-fuse-ld=mold -C link-arg=-lm" cargo build --release --target $PLATFORM -p rustfs --bins
%endif

%install
mkdir -p %buildroot/usr/bin/
install %_builddir/%{name}-%{version}-%{prerelease}/target/%_arch/%_arch-unknown-linux-gnu/release/zffs %buildroot/usr/bin/
ln -s zffs %buildroot/usr/bin/rustfs

%files
%license LICENSE
%doc docs
%_bindir/zffs
%_bindir/rustfs

%changelog
* Thu Jul 30 2026 overtrue <anzhengchao@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.12

* Thu Jul 23 2026 overtrue <anzhengchao@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.11

* Fri Jul 17 2026 overtrue <anzhengchao@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.10

* Thu Jul 16 2026 houseme <housemecn@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.10-preview.4

* Thu Jul 16 2026 houseme <housemecn@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.10-preview.1

* Tue Jul 14 2026 houseme <housemecn@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.9

* Wed Jun 10 2026 houseme <housemecn@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.8

* Wed Jun 03 2026 houseme <housemecn@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.7

* Thu May 28 2026 houseme <housemecn@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.6

* Thu May 20 2026 houseme <housemecn@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.4

* Thu May 14 2026 houseme <housemecn@gmail.com>
- Update RPM package to RustFS 1.0.0-beta.3

* Thu Jan 28 2026 houseme <housemecn@gmail.com>
- Initial RPM package for RustFS 1.0.0-alpha.81

* Thu Nov 20 2025 Wenlong Zhang <zhangwenlong@loongson.cn>
- Initial RPM package for RustFS 1.0.0-alpha.69

* Tue Jul 08 2025 Wenlong Zhang <zhangwenlong@loongson.cn>
- Initial RPM package for RustFS 1.0.0-alpha.36
