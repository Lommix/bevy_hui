#! /bin/bash

hui_tmp=$(mktemp -d)
widget_tmp=$(mktemp -d)

cp -r crates/bevy_hui/* "$hui_tmp"/.
cp -r LICENSE-MIT LICENSE-APACHE README.md "$hui_tmp"/.

cp -r crates/bevy_hui_widgets/* "$widget_tmp"/.
cp -r LICENSE-MIT LICENSE-APACHE "$widget_tmp"/.

sed -i 's|"../../../README.md"|"../README.md"|g' "$hui_tmp"/src/lib.rs

version=$(grep "^version" $hui_tmp/Cargo.toml | sed 's/version = "\(.*\)"/\1/')
sed -i '/^bevy_hui/c\bevy_hui={version="'${version}'",features=["picking"]}' "$widget_tmp"/Cargo.toml

cd $hui_tmp && cargo publish
cd $widget_tmp && cargo publish

cat "$widget_tmp"/Cargo.toml

rm -rf "$hui_tmp"
rm -rf "$widget_tmp"
