# YAML Schema

githops поставляется с JSON Schema для `githops.yaml`. При запуске `githops init` файл схемы записывается в `.githops/githops.schema.json`, а файл конфигурации получает заголовочный комментарий, указывающий редакторам на него.

## Интеграция с редактором

`githops.yaml` начинается с:

```yaml
# yaml-language-server: $schema=.githops/githops.schema.json
```

Этот комментарий распознаётся `yaml-language-server`, который обеспечивает YAML IntelliSense в VS Code (через расширение YAML от Red Hat), Neovim (через `nvim-lspconfig`) и IntelliJ IDEA. Вы получаете:

- Автодополнение для всех ключей (`hooks`, `definitions`, `cache`, имена хуков, поля команд).
- Встроенную документацию для каждого поля при наведении.
- Ошибки валидации для неизвестных полей или неправильных типов, отображаемые прямо в редакторе.

## Справочник по конфигурации

### Верхнеуровневая структура

```yaml
# yaml-language-server: $schema=.githops/githops.schema.json

hooks:
  <имя-хука>:
    enabled: true
    parallel: false
    commands: []

definitions:
  <имя-определения>:
    name: строка
    run: строка

cache:
  enabled: false
  dir: .githops/cache
```

### hooks

Словарь имён Git-хуков и их конфигураций. Поддерживаемые имена хуков — все стандартные Git-хуки: `pre-commit`, `prepare-commit-msg`, `commit-msg`, `post-commit`, `pre-push`, `pre-rebase`, `post-merge`, `post-checkout`, `post-rewrite`, `pre-merge-commit`, `pre-receive`, `update`, `post-receive`, `post-update` и другие.

#### Поля хука

| Поле | Тип | По умолчанию | Описание |
|---|---|---|---|
| `enabled` | boolean | `true` | Активен ли хук. Установите `false`, чтобы отключить без удаления конфигурации. |
| `parallel` | boolean | `false` | Запускать все команды этого хука параллельно. Команды с `depends` по-прежнему ждут зависимостей. |
| `commands` | array | `[]` | Упорядоченный список команд или ссылок на определения. |

#### Поля команды (встроенной)

| Поле | Тип | По умолчанию | Описание |
|---|---|---|---|
| `name` | string | обязательное | Уникальное имя в рамках этого хука. Используется в `depends`. |
| `run` | string | обязательное | Шелл-команда для выполнения. Запускается в `sh -c` на Unix, `cmd /c` на Windows. |
| `depends` | string[] | `[]` | Имена команд в том же хуке, которые должны успешно завершиться перед этой командой. |
| `env` | object | `{}` | Переменные окружения только для этой команды. |
| `test` | boolean | `false` | Если `true`, команда запускается только в тестовом режиме (например, `githops check --test`). |
| `cache` | object | — | Включить контентное кэширование для этой команды. См. [Кэширование](./caching). |

#### Ссылка на определение (`$ref`)

Вместо встроенной команды можно сослаться на определение:

```yaml
commands:
  - $ref: my-definition
    args: "--fix"
    name: "lint with fix"   # опциональное переопределение имени
```

| Поле | Тип | Описание |
|---|---|---|
| `$ref` | string | Имя используемого определения. |
| `args` | string | Дополнительные аргументы, добавляемые к команде `run` определения. |
| `name` | string | Переопределение отображаемого имени для данного использования определения. |

### definitions

Словарь переиспользуемых определений команд. Каждое определение может быть одной командой или списком команд.

**Одиночная команда:**

```yaml
definitions:
  lint:
    name: Запуск ESLint
    run: npx eslint .
    depends: []
    env: {}
    test: false
```

**Список команд:**

```yaml
definitions:
  setup:
    - name: install
      run: npm ci
    - name: build
      run: npm run build
      depends:
        - install
```

### cache

Глобальные настройки кэша.

| Поле | Тип | По умолчанию | Описание |
|---|---|---|---|
| `enabled` | boolean | `false` | Включить контентное кэширование глобально. |
| `dir` | string | `.githops/cache` | Директория для хранения маркерных файлов кэша. |

## Обновление схемы

Если вы обновили githops и хотите обновить файл схемы:

```sh
githops init
```

`init` безопасно запускать на существующем проекте — он только записывает файл схемы и не перезаписывает `githops.yaml`.
