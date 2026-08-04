# NovaiOS default bashrc-equivalent for Nushell
# This is loaded by /home/novai/.config/nushell/config.nu in the live ISO.

$env.PATH = ($env.PATH | append [
    "/usr/local/bin"
    "/usr/local/sbin"
    "/usr/bin"
    "/usr/sbin"
    "/bin"
    "/sbin"
    "/home/novai/.cargo/bin"
])

$env.EDITOR = "helix"
$env.VISUAL = "helix"
$env.SHELL  = "/usr/bin/nu"
$env.STARSHIP_CONFIG = "/etc/novai/starship.toml"

alias ll = eza -lah --git --icons
alias cat = bat --paging=never
alias find = fd
alias grep = rg
alias cd = zoxide
