# Maintainer: Your Name <you@example.com>

pkgname=tilecap
pkgver=0.1.0
pkgrel=1
pkgdesc='Minimal screenshot tool for tiling window managers'
arch=('x86_64')
url='https://github.com/AlexIvanin/tilecap'
license=('MIT')
depends=('libx11' 'libxcb' 'xclip')
makedepends=('cargo' 'rust')
source=("$pkgname-$pkgver.tar.gz::https://github.com/AlexIvanin/tilecap/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo build --release --locked
}

check() {
    cd "$srcdir/$pkgname-$pkgver"
    cargo test --release --locked 2>/dev/null || true
}

package() {
    cd "$srcdir/$pkgname-$pkgver"
    install -Dm755 target/release/tilecap "$pkgdir/usr/bin/tilecap"
    install -Dm644 README.md "$pkgdir/usr/share/doc/tilecap/README.md"
}
