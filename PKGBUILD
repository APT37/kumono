pkgbase=kumono
pkgname=('cli')
# pkgname=('cli' 'daemon')
pkgver=0.1.0
pkgrel=1
pkgdesc="Media ripper for coomer.su and kemono.su"
arch=('x86_64')
url="https://git.nospy.in/Rust/$pkgbase"

package_cli() {
  install -Dm755 "$startdir/target/release/$pkgname" "$pkgdir/usr/bin/$pkgbase-$pkgname"
}

# package_daemon() {
  # backup=("etc/init.d/$pkgbase-$pkgname")

  # install -Dm755 "$startdir/target/release/$pkgname" "$pkgdir/usr/bin/$pkgbase-$pkgname"
  # install -Dm755 "$startdir/$pkgname/$pkgname.rc" "$pkgdir/etc/init.d/$pkgbase-$pkgname"
# }