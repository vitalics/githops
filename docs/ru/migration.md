# Руководство по миграции

Это руководство охватывает миграцию на githops с Husky, lefthook и pre-commit. Общий подход одинаков для всех трёх: запустите `githops migrate` для генерации начального `githops.yaml` из существующей конфигурации, затем просмотрите и доработайте результат.

## Автоматическая миграция

```sh
githops migrate
```

githops определяет текущий менеджер хуков по конфигурационным файлам в корне репозитория:

- Директория `.husky/` → Husky
- `lefthook.yml` или `lefthook.yaml` → lefthook
- `.pre-commit-config.yaml` → pre-commit

Он читает существующую конфигурацию, преобразует её в `githops.yaml` и записывает файл. Существующие файлы менеджера хуков не удаляются.

Внимательно проверьте сгенерированный файл. Автоматическая миграция обрабатывает распространённые случаи, но может не охватить всё — например, условную логику в шелл-скриптах Husky нужно проверять вручную.

## Миграция с Husky

### Структура Husky v9

Типичный проект с Husky имеет файлы хуков в `.husky/`:

```sh
.husky/
  pre-commit    # шелл-скрипт
  pre-push      # шелл-скрипт
```

Каждый файл — это обычный шелл-скрипт:

```sh
# .husky/pre-commit
npm run lint
npm run typecheck
```

### Эквивалентный githops.yaml

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
```

### Шаги

1. Запустите `githops migrate` для генерации начального `githops.yaml`.
2. Просмотрите сгенерированный файл.
3. Запустите `githops sync` для установки хуков под управлением githops.
4. Удалите скрипт `prepare` из `package.json`, который устанавливал Husky, или замените его на `githops sync`.
5. Удалите `.husky/` и уберите `husky` из `devDependencies`.

```sh
# Удаление husky
npm uninstall husky
rm -rf .husky
```

## Миграция с lefthook

### Структура lefthook.yml

```yaml
pre-commit:
  parallel: true
  commands:
    lint:
      run: npm run lint
    typecheck:
      run: npm run typecheck

pre-push:
  commands:
    test:
      run: npm test
```

### Эквивалентный githops.yaml

```yaml
hooks:
  pre-commit:
    enabled: true
    parallel: true
    commands:
      - name: lint
        run: npm run lint
      - name: typecheck
        run: npm run typecheck

  pre-push:
    enabled: true
    commands:
      - name: test
        run: npm test
```

### Шаги

1. Запустите `githops migrate`.
2. Просмотрите `githops.yaml`.
3. Запустите `githops sync`.
4. Удалите lefthook из проекта.

```sh
# На macOS с Homebrew
brew uninstall lefthook
# Или если установлен через npm
npm uninstall @evilmartians/lefthook
```

## Миграция с pre-commit

pre-commit использует модель плагинов, где хуки ссылаются на внешние репозитории. githops не имеет системы плагинов — вы пишете команды напрямую. Миграция означает замену ссылок на плагины эквивалентными командами, установленными в проекте.

### Пример .pre-commit-config.yaml

```yaml
repos:
  - repo: https://github.com/pre-commit/mirrors-eslint
    rev: v8.56.0
    hooks:
      - id: eslint
  - repo: https://github.com/pre-commit/mirrors-prettier
    rev: v3.1.0
    hooks:
      - id: prettier
```

### Эквивалентный githops.yaml

```yaml
hooks:
  pre-commit:
    enabled: true
    commands:
      - name: eslint
        run: npx eslint .
      - name: prettier
        run: npx prettier --check .
```

### Шаги

1. Определите, какие команды запускает каждый плагин pre-commit. Большинство зеркал просто вызывают CLI инструмента.
2. Убедитесь, что эти инструменты установлены в проекте (`npm install --save-dev eslint prettier`).
3. Напишите эквивалентный `githops.yaml` вручную или запустите `githops migrate` для начальной точки.
4. Запустите `githops sync`.
5. Удалите pre-commit.

```sh
pip uninstall pre-commit
rm .pre-commit-config.yaml
```

## Контрольный список ручной миграции

После миграции с любого инструмента:

- [ ] `githops.yaml` существует и `githops sync` запускается без ошибок.
- [ ] `.git/hooks/` содержит хуки под управлением githops (проверьте `ls .git/hooks/`).
- [ ] Файлы старого менеджера хуков удалены из репозитория.
- [ ] Старый менеджер хуков удалён из `devDependencies` / системных пакетов.
- [ ] Скрипт `prepare` в `package.json` (если есть) вызывает `githops sync` вместо старого инструмента.
- [ ] `.gitignore` включает `.githops/cache`, если планируется использование кэширования.
- [ ] Члены команды уведомлены о необходимости запустить `githops sync` (или `npm install`, если используется скрипт `prepare`).
