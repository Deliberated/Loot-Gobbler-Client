# Built by .github/workflows/release.yml from the portable folder (the tar.gz's); the macros app_version, app_image,
# desktop_file and icon_file come from there.
#
# Installs into /opt/LootGobbler with a menu entry. %caps gives the program the capabilities capturing needs, at
# every install and update, so it never has to ask for them. The app keeps its data in ~/.local/share/LootGobbler,
# since /opt is read-only for the user (lg_app::files::own_folder).

%global debug_package %{nil}

Name:           lootgobbler
Version:        %{app_version}
Release:        1
Summary:        Loot Gobbler - your right hand for your RotMG adventures
License:        MIT
URL:            https://github.com/Deliberated/Loot-Gobbler-Client
BuildArch:      x86_64
Requires:       libpcap

%description
A read-only Realm of the Mad God companion: loot log, collection, characters, DPS logger, guild features and more.
It reads the game's network traffic and never sends, blocks or modifies game packets.

%install
mkdir -p %{buildroot}/opt
cp -a %{app_image} %{buildroot}/opt/LootGobbler
install -Dm644 %{desktop_file} %{buildroot}%{_datadir}/applications/lootgobbler.desktop
install -Dm644 %{icon_file} %{buildroot}%{_datadir}/icons/hicolor/256x256/apps/lootgobbler.png

%files
%dir /opt/LootGobbler
%caps(cap_net_raw,cap_net_admin=eip) /opt/LootGobbler/LootGobbler
/opt/LootGobbler/LICENSE
/opt/LootGobbler/THIRD_PARTY_LICENSE-RealmShark.md
/opt/LootGobbler/README.txt
%{_datadir}/applications/lootgobbler.desktop
%{_datadir}/icons/hicolor/256x256/apps/lootgobbler.png
