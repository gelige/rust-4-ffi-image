# Rust FFI Image Processor

Проект демонстрирует обработку изображений через динамически загружаемые плагины на Rust.

Основная программа `image_processor` читает PNG-файл, преобразует его в `RGBA8`, загружает динамическую библиотеку
плагина и вызывает экспортированную функцию:

```rust
extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
)
```

Плагин получает ширину, высоту, указатель на плоский массив RGBA-данных и строку параметров. Результат записывается в
тот же массив пикселей, после чего `image_processor` сохраняет PNG-файл.

## Плагины

- `plugin_mirror` - отражение изображения по горизонтали и/или вертикали.
- `plugin_blur` - размытие изображения с заданным радиусом и количеством итераций.

## Сборка

Собрать весь workspace:

```bash
cargo build
```

Собрать только плагины:

```bash
cargo build -p plugin_mirror
cargo build -p plugin_blur
```

После сборки динамические библиотеки будут находиться в `target/debug`.

Имена библиотек зависят от платформы:

- Linux: `libplugin_mirror.so`, `libplugin_blur.so`
- macOS: `libplugin_mirror.dylib`, `libplugin_blur.dylib`
- Windows: `plugin_mirror.dll`, `plugin_blur.dll`

## Запуск

Пример параметров для `plugin_mirror`, файл `mirror.json`:

```json
{
    "horizontal": true,
    "vertical": false
}
```

```bash
cargo run -p image_processor -- \
  --input input-sample.png \
  --output output.png \
  --plugin plugin_mirror \
  --plugin-path target/debug \
  --params mirror.json
```

Пример параметров для `plugin_blur`, файл `blur.json`:

```json
{
    "radius": 3,
    "iterations": 1
}
```

```bash
cargo run -p image_processor -- \
  --input input-sample.png \
  --output output.png \
  --plugin plugin_blur \
  --plugin-path target/debug \
  --params blur.json
```

Если `--params` не указан, в плагин передается пустой JSON-объект `{}`.