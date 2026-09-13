# Zed Razor

## Setup
First install [rustup](https://rustup.rs).
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
```
Then clone this repositry and install it as a "Dev Extension" in Zed. 
```bash
git clone https://github.com/bozidarsk/zed-razor
```
In `settings.json` you need to add where the razor source code (from `dotnet/roslyn`) will be installed and build - it is needed by the extension. Example:
```json
"lsp": {
    "roslyn-razor": {
        "settings": {
            "roslyn_source_repository_root": "~/.cache/dotnet-roslyn-src"
        }
    }
}
```
Or do it manually:
```bash
git clone https://github.com/dotnet/roslyn --branch release/stable ~/.cache/dotnet-roslyn-src
cd ~/.cache/dotnet-roslyn-src
dotnet build src/Razor/src/Razor/src/Microsoft.VisualStudioCode.RazorExtension/Microsoft.VisualStudioCode.RazorExtension.csproj -c Release
```
