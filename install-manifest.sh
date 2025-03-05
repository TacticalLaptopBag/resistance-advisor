#!/bin/bash


# EXE_PATH=`realpath ./target/debug/resistance-advisor`
EXE_PATH=/usr/local/bin/resistance-advisor
MANIFEST_NAME=com.github.tacticallaptopbag.resistance.json

cp ./native-messaging-manifest.template.json ./native-messaging-manifest.firefox.json
sed -i -e "s%\$EXE_PATH%$EXE_PATH%" ./native-messaging-manifest.firefox.json
sed -i -e "s%\$ALLOWED_EXT%allowed_extensions%" ./native-messaging-manifest.firefox.json
sed -i -e "s%\$EXT_ID%resistance-scanner@extension.js%" ./native-messaging-manifest.firefox.json

cp ./native-messaging-manifest.template.json ./native-messaging-manifest.chrome.json
sed -i -e "s%\$EXE_PATH%$EXE_PATH%" ./native-messaging-manifest.chrome.json
sed -i -e "s%\$ALLOWED_EXT%allowed_origins%" ./native-messaging-manifest.chrome.json
sed -i -e "s%\$EXT_ID%chrome-extension://*/%" ./native-messaging-manifest.chrome.json

sudo mkdir -p /usr/lib/librewolf/native-messaging-hosts /usr/lib/mozilla/native-messaging-hosts /etc/opt/chrome/native-messaging-hosts
sudo cp ./native-messaging-manifest.firefox.json /usr/lib/mozilla/native-messaging-hosts/$MANIFEST_NAME
sudo cp ./native-messaging-manifest.firefox.json /usr/lib/librewolf/native-messaging-hosts/$MANIFEST_NAME
sudo cp ./native-messaging-manifest.chrome.json /etc/opt/chrome/native-messaging-hosts/$MANIFEST_NAME
cp ./native-messaging-manifest.chrome.json ~/.var/app/io.github.ungoogled_software.ungoogled_chromium/config/chromium/NativeMessagingHosts/$MANIFEST_NAME

rm ./native-messaging-manifest.firefox.json ./native-messaging-manifest.chrome.json

