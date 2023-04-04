git clone --depth=1 https://github.com/romkatv/powerlevel10k.git ~/.oh-my-zsh/themes/powerlevel10k;
sed -i 's/devcontainers/powerlevel10k\/powerlevel10k/g' ~/.zshrc;
echo "\
source /workspaces/Compositor/.devcontainer/pl10k.zsh\n\
export PKG_CONFIG_PATH=\"/usr/lib/$(ls /usr/lib/|grep "linux-gnu")/pkgconfig\"\n\
export XDG_RUNTIME_DIR=/run/user/$(id -u)\n\
export DISPLAY=host.docker.internal:0
" >> ~/.zshrc;
su - root -c "curl https://sh.rustup.rs -sSf | sh -s -- -y --default-toolchain nightly"
mkdir -p /run/user/$(id -u)
chmod 0700 /run/user/$(id -u)
