FROM fedora:37

RUN dnf install -y rustc cargo rust-gdk4-sys-devel openssl-devel rust-libadwaita0.1-devel 

RUN dnf install -y zsh util-linux-user git
RUN chsh root -s /bin/zsh
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" --unattended

RUN groupadd -g 1000 build
RUN useradd -u 1000 -g 1000 -m build

RUN chsh build -s /bin/zsh
RUN mkdir /Depot

USER build
RUN sh -c "$(curl -fsSL https://raw.githubusercontent.com/ohmyzsh/ohmyzsh/master/tools/install.sh)" --unattended

WORKDIR /Depot
SHELL [ "/bin/zsh", "-l" ]
CMD [ "zsh" ]
