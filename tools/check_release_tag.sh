#!/bin/sh
# Сверяет тег релиза с версией пакета: `v7.2` и `v7.2.0` означают версию 7.2.0.
# Прежние теги проекта двухчастные (v7.1), а Cargo требует три числа, поэтому `vX.Y` читается как `X.Y.0`.
# Использование: check_release_tag.sh <тег> <версия из Cargo.toml>
# Код возврата 0 — тег подходит; иначе 1 и сообщение для журнала GitHub Actions.

tag=$1
version=$2

case "$tag" in
  v[0-9]*.[0-9]*.[0-9]*) expected=${tag#v} ;;
  v[0-9]*.[0-9]*) expected=${tag#v}.0 ;;
  *)
    echo "::error::тег '$tag' не похож на версию: нужен vX.Y или vX.Y.Z"
    exit 1
    ;;
esac

if [ "$expected" != "$version" ]; then
  echo "::error::тег '$tag' означает версию $expected, а в Cargo.toml указана $version"
  exit 1
fi
echo "тег $tag соответствует версии $version"
