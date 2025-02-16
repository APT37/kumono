pkgname=coomer-rip
pkgver=0.5.0
pkgrel=2
pkgdesc="Helper tool for ripping media from coomer.su creator posts"
arch=('x86_64')
url="https://git.nospy.in/Rust/$pkgname"

package() {
  install -Dm755 "$startdir/target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
}