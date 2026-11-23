//! Linux: what capturing needs and how to get it (Java `LinuxCaptureGUI`). libpcap to read the traffic, and the right
//! to capture (`setcap` gives it to the program). With the system's password prompt (`pkexec`) one prompt installs
//! what is missing and gives the right; without it, the commands for a terminal.

use std::path::Path;

/// The rights capturing needs.
pub const CAPS: &str = "cap_net_raw,cap_net_admin=eip";
const PKEXEC: &str = "/usr/bin/pkexec";
/// Where the programs are looked for (also the PATH the setup script runs with).
const PATH: &str = "/usr/sbin:/usr/bin:/sbin:/bin";

/// A distribution's package manager: how it installs, and its packages for libpcap and for `setcap`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packages {
    pub name: &'static str,
    install: &'static str,
    /// Alternatives tried in turn (Debian and Ubuntu renamed the library).
    pcap: &'static [&'static str],
    setcap: &'static str,
}

const MANAGERS: [(&str, Packages); 4] = [
    (
        "apt-get",
        Packages { name: "Debian / Ubuntu", install: "apt-get install -y", pcap: &["libpcap0.8t64", "libpcap0.8"], setcap: "libcap2-bin" },
    ),
    ("dnf", Packages { name: "Fedora", install: "dnf install -y", pcap: &["libpcap"], setcap: "libcap" }),
    ("zypper", Packages { name: "openSUSE", install: "zypper --non-interactive install", pcap: &["libpcap1"], setcap: "libcap-progs" }),
    ("pacman", Packages { name: "Arch", install: "pacman -S --noconfirm --needed", pcap: &["libpcap"], setcap: "libcap" }),
];

fn find(program: &str) -> bool {
    PATH.split(':').any(|d| Path::new(d).join(program).is_file())
}

/// What this system has and lacks.
#[derive(Debug, Clone)]
pub struct Plan {
    /// libpcap has to be installed.
    pub pcap: bool,
    /// `setcap` has to be installed.
    pub setcap: bool,
    pub pkexec: bool,
    pub packages: Option<Packages>,
}

impl Plan {
    pub fn now(pcap_missing: bool) -> Plan {
        Plan {
            pcap: pcap_missing,
            setcap: !find("setcap"),
            pkexec: Path::new(PKEXEC).is_file(),
            packages: MANAGERS.iter().find(|(pm, _)| find(pm)).map(|(_, p)| *p),
        }
    }

    /// The names of what is installed, for the dialog ("libpcap and setcap").
    pub fn missing(&self) -> Vec<&'static str> {
        [(self.pcap, "libpcap"), (self.setcap, "setcap")].into_iter().filter(|(m, _)| *m).map(|(_, n)| n).collect()
    }

    /// One password prompt can do it all: the prompt is there, and what is missing can be installed.
    pub fn by_itself(&self) -> bool {
        self.pkexec && (self.missing().is_empty() || self.packages.is_some())
    }

    /// The install commands (without sudo), one per missing part.
    fn installs(&self) -> Vec<String> {
        let Some(p) = self.packages else { return Vec::new() };
        let mut out = Vec::new();
        if self.pcap {
            out.push(p.pcap.iter().map(|pkg| format!("{} {pkg}", p.install)).collect::<Vec<_>>().join(" || "));
        }
        if self.setcap {
            out.push(format!("{} {}", p.install, p.setcap));
        }
        out
    }

    /// The script the password prompt runs as root: install what is missing, then give the program the right.
    pub fn script(&self, exe: &Path) -> String {
        let mut steps: Vec<String> = self.installs().into_iter().map(|c| format!("( {c} )")).collect();
        steps.push(format!("setcap {CAPS} {}", quote(exe)));
        format!("export PATH={PATH}; {}", steps.join(" && "))
    }

    /// The commands for a terminal, when the app cannot do it by itself.
    pub fn manual(&self, exe: &Path) -> String {
        let mut lines = Vec::new();
        match self.packages {
            Some(_) => lines.extend(self.installs().into_iter().map(|c| format!("sudo {}", c.replace(" || ", " || sudo ")))),
            None => {
                let names = self.missing();
                if !names.is_empty() {
                    lines.push(format!("# install {} with your package manager", names.join(" and ")));
                }
            }
        }
        lines.push(format!("sudo setcap {CAPS} {}", quote(exe)));
        lines.join("\n")
    }

    /// Runs the script through the password prompt (blocks until it is answered).
    pub fn run(&self, exe: &Path) -> Result<(), String> {
        let o = std::process::Command::new(PKEXEC).args(["/bin/sh", "-c", &self.script(exe)]).output().map_err(|e| e.to_string())?;
        match o.status.code() {
            Some(0) => Ok(()),
            // pkexec: 126 the password prompt was closed, 127 not authorized
            Some(126) => Err("The password prompt was closed.".into()),
            Some(127) => Err("Not authorized.".into()),
            code => {
                let err = String::from_utf8_lossy(&o.stderr);
                let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("").trim().to_string();
                Err(if last.is_empty() { format!("The setup failed ({code:?}).") } else { last })
            }
        }
    }
}

/// A path as typed in a shell.
pub fn quote(path: &Path) -> String {
    let s = path.display().to_string();
    if s.chars().all(|c| c.is_ascii_alphanumeric() || "_./+-".contains(c)) { s } else { format!("'{}'", s.replace('\'', "'\\''")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(pcap: bool, setcap: bool, pm: usize) -> Plan {
        Plan { pcap, setcap, pkexec: true, packages: Some(MANAGERS[pm].1) }
    }

    #[test]
    fn only_the_right_when_all_is_there() {
        let p = plan(false, false, 0);
        assert!(p.by_itself());
        assert_eq!(p.script(Path::new("/opt/LG/LootGobbler")), format!("export PATH={PATH}; setcap {CAPS} /opt/LG/LootGobbler"));
        assert_eq!(p.manual(Path::new("/opt/LG/LootGobbler")), format!("sudo setcap {CAPS} /opt/LG/LootGobbler"));
    }

    #[test]
    fn installs_what_is_missing_first() {
        let p = plan(true, true, 0);
        assert_eq!(p.missing(), ["libpcap", "setcap"]);
        assert_eq!(
            p.script(Path::new("/home/me/Loot Gobbler/LootGobbler")),
            format!(
                "export PATH={PATH}; ( apt-get install -y libpcap0.8t64 || apt-get install -y libpcap0.8 ) && ( apt-get install -y libcap2-bin ) && setcap {CAPS} '/home/me/Loot Gobbler/LootGobbler'"
            )
        );
        assert_eq!(
            plan(false, true, 1).manual(Path::new("/x/LootGobbler")),
            format!("sudo dnf install -y libcap\nsudo setcap {CAPS} /x/LootGobbler")
        );
    }

    #[test]
    fn unknown_distributions_get_told() {
        let p = Plan { pcap: false, setcap: true, pkexec: true, packages: None };
        assert!(!p.by_itself());
        assert!(p.manual(Path::new("/x/LG")).starts_with("# install setcap with your package manager\n"));
        assert!(Plan { setcap: false, ..p }.by_itself());
    }
}
