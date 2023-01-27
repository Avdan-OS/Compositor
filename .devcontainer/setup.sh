git clone --depth=1 https://github.com/romkatv/powerlevel10k.git ~/.oh-my-zsh/themes/powerlevel10k;
sed -i 's/devcontainers/powerlevel10k\/powerlevel10k/g' ~/.zshrc;
echo "\
source /workspaces/Compositor/.devcontainer/pl10k.zsh \n\
export PKG_CONFIG_PATH=\"/usr/lib/aarch64-linux-gnu/pkgconfig\"\
" >> ~/.zshrc;
curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly
