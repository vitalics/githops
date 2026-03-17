# Внешние импорты

githops может импортировать скрипты из внешних файлов конфигурации — локальных файлов репозитория, файлов по HTTP или файлов из удалённых Git-репозиториев. Это позволяет хранить общие скрипты в одном месте и переиспользовать их в разных проектах.

## Объявление импорта

Все импорты перечисляются в ключе `include:` на верхнем уровне. У каждого элемента есть поле `source` (`local`, `remote` или `git`) и поле `ref` — имя импорта, по которому он используется в командах хуков через `$include:`.

### Локальный файл

Импортируйте скрипты из файла репозитория (`package.json`, `Cargo.toml`, YAML со скриптами и т.д.):

```yaml
include:
  - source: local
    path: package.json
    type: json
    ref: packagejson
```

Поддерживаемые типы: `json`, `toml`, `yaml`.

### Удалённый файл

Загрузка файла по HTTP или HTTPS:

```yaml
include:
  - source: remote
    url: 'https://example.com/shared-scripts.yaml'
    type: yaml
    ref: sharedscripts
```

Файл загружается при каждом запуске хука. Поле `type` по умолчанию равно `yaml`.

### Git-репозиторий

Чтение одного файла из удалённого Git-репозитория на определённой ревизии:

```yaml
include:
  - source: git
    url: 'https://github.com/org/repo.git'
    rev: main
    file: 'ci/scripts.yaml'
    type: yaml
    ref: repotemplate
```

githops выполняет `git clone --depth=1` во временную директорию и читает указанный файл. Поле `type` по умолчанию равно `yaml`.

## Использование импорта в хуке

Ссылайтесь на импорт в командах хука через `$include:`:

```yaml
hooks:
  pre-commit:
    enabled: true
    commands:
      - $include: packagejson
        run: scripts.lint
```

Поле `run` — это путь в нотации с точками к нужному значению в файле. githops находит значение по этому пути и использует его как команду для выполнения.

Например, если `package.json` содержит:
```json
{
  "scripts": {
    "lint": "eslint . --ext .ts"
  }
}
```

То `run: scripts.lint` разрешается в `eslint . --ext .ts`.

### Дополнительные аргументы

Используйте `args` для добавления флагов к разрешённой команде:

```yaml
- $include: packagejson
  run: scripts.lint
  args: "--fix"
```

Это даёт `eslint . --ext .ts --fix`.

### Переменные окружения

Используйте `env` для установки переменных окружения:

```yaml
- $include: packagejson
  run: scripts.lint
  args: "--fix"
  env:
    NODE_ENV: production
```

### Имя для отображения

Добавьте поле `name`, чтобы переопределить метку в выводе и графе:

```yaml
- $include: packagejson
  run: scripts.lint
  name: ESLint
```

Без `name` githops использует последний сегмент пути (`lint` в данном случае).

## Поддерживаемые форматы

| Тип    | Примеры файлов         | Навигация                             |
|--------|------------------------|---------------------------------------|
| `json` | `package.json`         | `scripts.lint`, `dependencies.react`  |
| `toml` | `Cargo.toml`           | `package.version`, `scripts.build`    |
| `yaml` | `scripts.yaml`         | `jobs.lint.script`, `scripts.build`   |

Значение по найденному пути должно быть строкой.

## Полный пример

```yaml
include:
  - source: local
    path: package.json
    type: json
    ref: pkg

  - source: remote
    url: 'https://example.com/shared.yaml'
    ref: shared

  - source: git
    url: 'https://github.com/org/hooks.git'
    rev: v2.1.0
    file: 'hooks/common.yaml'
    ref: common

hooks:
  pre-commit:
    enabled: true
    parallel: true
    commands:
      - $include: pkg
        run: scripts.lint
        args: "--fix"
        name: lint
      - $include: pkg
        run: scripts.typecheck
        name: typecheck
      - $include: shared
        run: scripts.format-check
        env:
          NODE_ENV: production

  pre-push:
    enabled: true
    commands:
      - $include: common
        run: scripts.test
```

## Примечания

- Удалённые и git-импорты требуют доступа к сети во время выполнения хука.
- Git-импорты требуют установленного `git` в системе.
- Временная директория для git-клонов: `{system-temp}/githops-git-{hash}`. Она переиспользуется в рамках одного запуска githops.
- Импорты видны и редактируемы на вкладке **Импорты** в `githops graph`.
