pkgname=coomer-rip
pkgver=0.18.1
pkgrel=1
pkgdesc="Media ripper for coomer.su and kemono.su"
arch=('x86_64')
url="https://git.nospy.in/Rust/$pkgname"

package() {
  install -Dm755 "$startdir/target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
}
