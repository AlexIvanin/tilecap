# tilecap

Скриншотер для тайловых WM. X11.

```
tilecap -m full              # весь экран → файл
tilecap -m window            # тык в окно → файл
tilecap -m region            # выделить область → файл
tilecap -m region -o clipboard
tilecap -r 800x600+100+50
```

## Установка

```
cargo build --release
install -m755 target/release/tilecap ~/.local/bin/
```

Пакеты: `make deb`, `make rpm`, в `gentoo/` и `PKGBUILD` лежат заготовки.

Зависимости: `xclip` для буфера обмена.

## Использование

```
-m full|window|region    режим
-o file|clipboard        куда сохранять
-d <dir>                 директория
-n <name>                имя файла
-r <WxH+X+Y>             регион без интерактива
-c <path>                конфиг
```

Конфиг: `~/.config/tilecap/config.toml`
```toml
output_dir = "/home/user/Pictures/Screenshots"
```

MIT.
