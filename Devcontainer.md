# Using devcontainer

## Why?

Normally, the compositor can only run on Linux. By leveraging the power of Docker, it could also be debugged on other operating systems. If you're on Windows or MacOS, use this guide to test or debug the compositor.

## Prerequisites

To use devcontainer, you need the following:

- Docker
- VSCode

In addition, you will need a xserver to display the compositor.

>  If you are using NVim, you might need to install and launch NVim in the container manually. Efforts are being made to make this process more automatic.

**MacOS**

You need XQuartz:

```sh
brew install --cask xquartz
```

and you need to allow docker to access XQuartz by

1. Open XQuartz
2. Open settings \(through menubar or `cmd`+`,`)
3. Go to security
4. Enable `Allow connections from network clients`

**Windows**

Hi, Frox here. I don't use Windows, so I would really appreciate if someone can help me with this part of the guide.

**Linux**

You probably already have xhost, so run the following command to allow docker to access xhost:

```sh
xhost +local:
```

If xhost is not found, you need to install it:

**Ubuntu and Debian**:

```sh
sudo apt-get install x11-xserver-utils
```

**CentOS and Fedora**:

```sh
sudo dnf install xorg-x11-xauth
```

**Arch Linux**:

```sh
sudo pacman -S xorg-xhost
```

## Running the devcontainer

1. Open the compositor project in VSCode.
2. Open command and search for `Dev Containers: Reopen in Container`
3. Installation and configuration will start automatically
4. Enjoy!

Now if you want to run the compositor, running `cargo run` should in theory just run. That, however, may have some compatibility issues. Happy coding!
