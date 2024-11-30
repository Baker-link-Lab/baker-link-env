# Baker link. Env (Baker link. Dev tool)

[<img alt="github" src="https://img.shields.io/badge/github-emilk/egui-8da0cb?logo=github" height="20">](https://github.com/emilk/egui)

<a href="https://www.buymeacoffee.com/Bakerlink.Lab" target="_blank"><img src="https://cdn.buymeacoffee.com/buttons/v2/default-yellow.png" alt="Buy Me A Coffee" style="height: 60px !important;width: 217px !important;" ></a>

<div align="center">

![Baker link](image/BakerLink-Orangeititlelpgp-1-300x44.png)

Development Environment Auxiliary Tools for the Baker link. series

</div>

![Baker link](image/Baker%20link.%20Env.001.png)

Portable Rust embedded development by communicating with Dev Containers (VSCode) and probe-rs DAP Server.
Baker link. Env is responsible for creating the project with embedded Dev Containers and starting probe-rs.

## Baker link. Env

![home_image](image/home_win.png)

### Required Tools

To use Baker link.Dev, the following tools must be installed.

- Docker ( [Rancher Desktop by SUSE](https://rancherdesktop.io/) )
- [Visual Studio Code](https://code.visualstudio.com/)
- [probe-rs](https://probe.rs/)

## Install

Compatible with both Windows and Mac.
There is an installer on the [Release](https://github.com/Baker-link-Lab/baker-link-env/releases) page.

### How to Address "Damaged" Error When Launching the App on Mac

After installing Baker link. Env, please run the following command in the terminal:

```sh
xattr -d com.apple.quarantine "/Applications/Baker link. Env.app"
```

## License

Copyright (c) 2024 Baker-Tanaka

Licensed under the [MIT](LICENSE) license.
