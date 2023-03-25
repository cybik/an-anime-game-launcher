FROM fedora:37

RUN dnf install -y rustc cargo rust-gdk4-sys-devel openssl-devel rust-libadwaita0.1-devel 

RUN dnf install -y zsh util-linux-user git
RUN chsh root -s /bin/zsh
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" --unattended

WORKDIR /Depot

RUN dnf install -y openssh-server openssh-clients lsof

# Simulate VSCode's installer
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/cybik/an-anime-game-launcher/aagl-steammode-horrors/25mar23/.devcontainer/sshstrap.sh)"
