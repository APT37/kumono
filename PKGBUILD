pkgname=kumono
pkgver=0.35.3
pkgrel=1
pkgdesc='Media ripper for coomer.su and kemono.su'
arch=('x86_64')
makedepends=('git' 'cargo' 'clang' 'mold')
url="https://github.com/APT37/$pkgbase"
license=('MIT-0')

package() {
    install -Dm755 "$startdir/target/release/$pkgname" -t "$pkgdir/usr/bin"
}