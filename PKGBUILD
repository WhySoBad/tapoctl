pkgname="tapoctl-git"
pkgver=v0.1.0.r2.g226af7a
pkgrel=1
pkgdesc="grpc server and cli for tapo bulbs"
arch=(x86_64)
url="https://github.com/WhySoBad/tapoctl"
license=(MIT)
depends=()
makedepends=(cargo-nightly protobuf)
source=("$pkgname::git+https://github.com/WhySoBad/tapoctl.git")
md5sums=('SKIP')
options=(!lto)

pkgver() {
    cd "$pkgname"
    git describe --long --abbrev=7 --tags | sed 's/\([^-]*-g\)/r\1/;s/-/./g'
}

prepare() {
    cd "$pkgname"

    export RUSTUP_TOOLCHAIN=nightly
    cargo fetch --locked --target "$(rustc -vV | sed -n 's/host: //p')"
}

build() {
    cd "$pkgname"
    export RUSTUP_TOOLCHAIN=nightly

    cargo build --frozen --release
}

package() {
    cd "$pkgname"

    install -Dm0755 "target/release/tapoctl" "$pkgdir/usr/bin/tapoctl"
}
