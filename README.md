# Spidey

Your friendly neighbourhood world wide web-crawler is here to re-imagine how you work and play on the web! Use a central panel to launch and control seamless Web Windows so you can focus on long tasks distraction-free and enjoy media without interruption.

![Screenshot1](https://github.com/kdwk/Spidey/blob/8da57713b323668dafd2a3aba9b4180f8b925340/data/resources/screenshots/Screenshot1.png)
![Screenshot2](https://github.com/kdwk/Spidey/blob/8da57713b323668dafd2a3aba9b4180f8b925340/data/resources/screenshots/Screenshot2.png)
![Screenshot3](https://github.com/kdwk/Spidey/blob/8da57713b323668dafd2a3aba9b4180f8b925340/data/resources/screenshots/Screenshot3.png)

## Get Spidey
### Pre-built pre-release
1. Enable the Gnome Nightly Flatpak repository if you haven't already ([instructions](https://wiki.gnome.org/Apps/Nightly))
2. Sign in to GitHub
3. Download the latest Nightly build [here](https://nightly.link/kdwk/Spidey/workflows/ci/main/spidey-x86_64.zip) (branch `main`, architecture `x86_64`)
4. Open the downloaded Flatpak file
5. Click "Install"
   
OR

3. Open Terminal and navigate to the directory to which the Flatpak file is downloaded
4. Run `flatpak install --user spidey.flatpak`

### Build from source
1. Enable the Gnome Nightly Flatpak repository if you haven't already ([instructions](https://wiki.gnome.org/Apps/Nightly))
2. Install [Builder](https://apps.gnome.org/Builder/) from Software
3. Click on the green "Code" button on this page and copy the URL
4. In Builder, click on "Clone Repository", paste the URL and click on "Clone Repository"
5. Wait for Builder to resolve SDK extensions, install missing runtimes if prompted
6. To run without installing, click the Play button
7. To export a Flatpak package for installation,
   1. Click Build button
   2. When it is done, open the drop-down beside the Build button and click "Export"
   3. When it is done, a Files window should automatically open with the exported Flatpak file
   4. Open the Flatpak file and click "Install"
