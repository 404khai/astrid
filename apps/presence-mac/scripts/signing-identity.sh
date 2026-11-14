#!/bin/sh
# Makes a private, self-signed code-signing identity for Astrid Presence in
# its own keychain (~/.astrid/signing; not the login keychain). bundle.sh
# signs with it when present, so the designated requirement is
# "identifier dev.astrid.presence and certificate leaf = <this cert>" and
# Screen Recording / Accessibility grants survive rebuilds. Run once.
set -eu
D="$HOME/.astrid/signing"
KC="$D/astrid-dev.keychain-db"
[ -f "$KC" ] && { echo "already set up: $KC"; exit 0; }
mkdir -p "$D" && cd "$D"
cat > cert.cnf <<'CNF'
[req]
distinguished_name=dn
x509_extensions=ext
prompt=no
[dn]
CN=Astrid Presence Dev
[ext]
basicConstraints=critical,CA:false
keyUsage=critical,digitalSignature
extendedKeyUsage=critical,codeSigning
CNF
openssl req -x509 -newkey rsa:2048 -nodes -keyout key.pem -out cert.pem -days 3650 -config cert.cnf 2>/dev/null
PASS=$(openssl rand -hex 16); echo "$PASS" > keychain.pass; chmod 600 keychain.pass key.pem
openssl pkcs12 -export -legacy -inkey key.pem -in cert.pem -out id.p12 -passout pass:astrid 2>/dev/null \
  || openssl pkcs12 -export -inkey key.pem -in cert.pem -out id.p12 -passout pass:astrid
security create-keychain -p "$PASS" "$KC"
security set-keychain-settings "$KC"
security unlock-keychain -p "$PASS" "$KC"
security import id.p12 -k "$KC" -P astrid -T /usr/bin/codesign >/dev/null
security set-key-partition-list -S apple-tool:,apple: -s -k "$PASS" "$KC" >/dev/null
rm -f id.p12
echo "made $KC"
