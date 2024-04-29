#!/bin/sh
@echo off
# Autogenerates a release for the current branch (a folder with all the shit we care about in it)
# Relies on this being windows, and 7z being installed, alongside wsl being setup on the system (for linux distros)
# with rust installed on both windows/wsl and wsl having musl-tools (it's required to make static binaries with the dependancies we have)
# I realize this is a bit of a "it works for me" kinda solution, but I figure it's useful to document the targets I use to generate releases
# also this way when I forget how to do this I'll have a nice tool for myself

rustup target add x86_64-pc-windows-msvc
wsl --shell-type login rustup target add x86_64-unknown-linux-musl
# Creates a release based off our current branch. Takes the release version as an arg
# First, let's build our binaries. Needs to be done statically so people w/o the right c runtime can vibe
cargo build --release --target x86_64-pc-windows-msvc
wsl --shell-type login cargo build --release --target x86_64-unknown-linux-musl

# Next, we'll create a folder to hold our shit
set name="Hypnagogic Release v%1"
mkdir %name%
cd %name%
# sets up gitignore so this doesn't pollute my life
echo * > .gitignore
# copy over the 2 binaries we just generated
copy ..\target\x86_64-pc-windows-msvc\release\hypnagogic-cli.exe .
copy ..\target\x86_64-unknown-linux-musl\release\hypnagogic-cli .
rename "hypnagogic-cli.exe" "hypnagogic.exe"
rename "hypnagogic-cli" "hypnagogic"

# make our to be zipped subfolder
mkdir hypnagogic-full-package
# insert a copy of the binaries and other shit we care about
copy hypnagogic.exe hypnagogic-full-package\
copy hypnagogic hypnagogic-full-package\
copy ..\LICENSE.md hypnagogic-full-package\
copy ..\README.md hypnagogic-full-package\
xcopy ..\examples hypnagogic-full-package\examples\ /E/H
xcopy ..\templates hypnagogic-full-package\templates\ /E/H

# finally zip up our package, yeah?
7z a hypnagogic-full-package.zip .\hypnagogic-full-package\*
tar -czf hypnagogic-full-package.tar.gz .\hypnagogic-full-package\*
# back out to normal
cd ..
