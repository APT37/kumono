pkgbase=kumono
pkgname=('kumono-cli')
# pkgname=('kumono-cli' 'kumono-daemon')
pkgver=0.11.0
pkgrel=1
pkgdesc="Media ripper for coomer.su and kemono.su"
arch=('x86_64')
url="https://git.nospy.in/Rust/$pkgbase"

package_kumono-cli() {
  install -Dm755 "$startdir/target/release/cli" "$pkgdir/usr/bin/$pkgname"
}

# package_kumono-daemon() {
  # backup=("etc/init.d/$pkgname")

  # install -Dm755 "$startdir/target/release/daemon" "$pkgdir/usr/bin/$pkgname"
  # install -Dm755 "$startdir/daemon/$pkgname.rc" "$pkgdir/etc/init.d/$pkgname"
# }