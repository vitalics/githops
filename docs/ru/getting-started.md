# Начало работы

## Установка

### macOS

Рекомендуемый способ установки githops на macOS — через `.pkg`-установщик, который автоматически помещает бинарный файл в `/usr/local/bin`.

```sh
# Apple Silicon (M1 и новее)
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-aarch64-apple-darwin.pkg -o githops.pkg
sudo installer -pkg githops.pkg -target /

# Intel
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-x86_64-apple-darwin.pkg -o githops.pkg
sudo installer -pkg githops.pkg -target /
```

### Linux

```sh
# x86_64
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv githops /usr/local/bin/

# ARM64
curl -fsSL https://github.com/vitalics/githops/releases/latest/download/githops-latest-aarch64-unknown-linux-gnu.tar.gz | tar xz
sudo mv githops /usr/local/bin/
```

### Windows

Скачайте и запустите MSI-установщик со [страницы релизов](https://github.com/vitalics/githops/releases/latest). Он устанавливает `githops.exe` и автоматически добавляет его в системный `PATH`.

### Cargo

Если установлен Rust-тулчейн:

```sh
cargo install githops
```

### npm / pnpm

githops будет опубликован в npm для использования в JavaScript-проектах:

```sh
# npm
npm install --save-dev githops

# pnpm
pnpm add --save-dev githops
```

После установки через npm/pnpm добавьте скрипт `prepare` в `package.json`, чтобы хуки устанавливались автоматически при запуске `npm install` членами команды:

```json
{
  "scripts": {
    "prepare": "githops sync"
  }
}
```

## Проверка установки

```sh
githops --version
```

## Инициализация репозитория

Перейдите в корень любого Git-репозитория и выполните:

```sh
githops init
```

Это создаст `githops.yaml` с примером конфигурации и запишет файл `.githops/githops.schema.json`, который включает YAML IntelliSense в VS Code и других редакторах с поддержкой протокола `yaml-language-server`.

## Первый хук

Откройте `githops.yaml` и добавьте хук `pre-commit`:

```yaml
# yaml-language-server: $schema=.githops/githops.schema.json

hooks:
  pre-commit:
    enabled: true
    commands:
      - name: lint
        run: npm run lint
      - name: typecheck
        run: npm run typecheck
        depends:
          - lint
```

Поле `depends` говорит githops запускать `typecheck` только после успешного завершения `lint`. Команды без зависимостей запускаются первыми; команды с зависимостями ждут их завершения.

## Установка хуков

```sh
githops sync
```

Эта команда записывает фактические скрипты хуков в `.git/hooks/`. Нужно запускать `sync` после изменения `githops.yaml`. Если добавить `githops sync` в скрипт `prepare` (см. раздел npm выше), это происходит автоматически.

## Проверка хука

Внесите изменение в отслеживаемый файл и создайте коммит:

```sh
git add .
git commit -m "test"
```

githops запустит хук `pre-commit`. Если какая-либо команда завершится с ненулевым кодом, коммит будет прерван.

## Проверка обновлений

```sh
githops self-update --check
```

Для установки последней версии:

```sh
githops self-update
```

## Автодополнение в шелле

Установите автодополнение для текущего шелла:

```sh
githops completions init
```

Это запишет скрипт автодополнения и добавит необходимые строки в rc-файл вашего шелла (`~/.zshrc` или `~/.bashrc`). Перезапустите терминал или выполните source rc-файла для активации.

## Дальнейшие шаги

- Прочитайте раздел [Функции](./yaml-schema), чтобы узнать о YAML Schema, шаблонах, параллелизации, визуальном графе и кэшировании.
- Если вы переходите с Husky, lefthook или pre-commit, см. [Руководство по миграции](./migration).
