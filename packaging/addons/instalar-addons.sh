#!/bin/sh
# Instala os addons da comunidade (Comunidade.xpx) no Core Rules para Linux.
#
# Uso: ./instalar-addons.sh [-y] [PASTA_DA_INSTALACAO]
#
# Sem PASTA_DA_INSTALACAO usa o pacote de dados do aplicativo
# ($XDG_DATA_HOME/corerules/data, senão ~/.local/share/corerules/data).
# Antes de importar, guarda UserDbas/ e Chars/ num .tar.gz ao lado da pasta.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
package="$here/Comunidade.xpx"
cr="$here/bin/cr"
yes=0

while [ $# -gt 0 ]; do
    case $1 in
        -y) yes=1; shift ;;
        -h|--help) sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        -*) echo "opção desconhecida: $1" >&2; exit 2 ;;
        *) break ;;
    esac
done

if [ $# -gt 0 ]; then
    dir=$1
else
    dir=${XDG_DATA_HOME:-$HOME/.local/share}/corerules/data
fi

die() { echo "erro: $*" >&2; exit 1; }

[ -f "$package" ] || die "não achei $package"
[ -x "$cr" ] || die "não achei $cr"
[ -d "$dir" ] || die "$dir não existe; crie o pacote de dados com ./extract-data.sh --install PASTA_DO_CORE_RULES (veja o README.md) ou informe a pasta"
dir=$(cd "$dir" && pwd)

# Acha um caminho dentro da instalação sem ligar para maiúsculas.
find_ci() { find "$dir" -maxdepth 2 -ipath "$dir/$1" 2>/dev/null | head -n 1; }

userdbas=$(find_ci UserDbas)
[ -n "$(find_ci Database)" ] || die "$dir não é uma instalação do Core Rules (falta Database/)"
[ -n "$userdbas" ] && [ -n "$(find_ci UserDbas/AllowedU.dat)" ] ||
    die "$dir não é da Expansion (falta UserDbas/AllowedU.dat); os addons são para a Expansion"

case $dir in
    */drive_c/*) wine=1 ;;
    *) wine=0 ;;
esac
if [ $wine -eq 1 ] && [ $yes -eq 0 ]; then
    echo "$dir é a instalação do programa original dentro do Wine."
    printf "Importar os addons nela mesmo assim? [s/N] "
    read -r answer
    case $answer in s|S|sim|Sim) ;; *) echo "nada foi alterado"; exit 1 ;; esac
fi

# Cópia de segurança dos registros e personagens do usuário.
stamp=$(date +%Y%m%d-%H%M%S)
backup="$(dirname "$dir")/$(basename "$dir")-antes-dos-addons-$stamp.tar.gz"
chars=$(find_ci Chars)
(
    cd "$dir"
    set -- "$(basename "$userdbas")"
    [ -n "$chars" ] && set -- "$@" "$(basename "$chars")"
    tar -czf "$backup" "$@"
)
echo "cópia de segurança: $backup"

log="$(dirname "$dir")/$(basename "$dir")-addons-$stamp.log"
marker="$userdbas/.addons-comunidade"
list=$(mktemp)
trap 'rm -f "$list"' EXIT

if [ -f "$marker" ]; then
    # Já instalado antes: importa tudo menos os personagens, que viriam em
    # dobro (cada personagem importado vira um arquivo novo em Chars/).
    echo "reinstalando (os personagens já foram importados antes)..."
    "$cr" import "$dir" "$package" --list |
        awk -F'\t' '$1 != "Characters" && $1 !~ /^\(/ { print $1 ":" $2 }' > "$list"
    tr '\n' '\0' < "$list" | xargs -0 "$cr" import "$dir" "$package" > "$log" 2>&1
else
    echo "importando $(basename "$package") (alguns segundos)..."
    "$cr" import "$dir" "$package" > "$log" 2>&1
fi

imported=$(grep -c '^imported ' "$log" || true)
characters=$(grep -c '^wrote Chars/' "$log" || true)

# Segunda passagem só com as classes: uma combinação raça/classe que cita
# duas classes do pacote só entra quando as duas já existem.
"$cr" import "$dir" "$package" --list |
    awk -F'\t' '$1 == "Classes" { print $1 ":" $2 }' > "$list"
echo "segunda passagem: $(wc -l < "$list") classes..."
echo "=== segunda passagem (classes)" >> "$log"
tr '\n' '\0' < "$list" | xargs -0 "$cr" import "$dir" "$package" >> "$log" 2>&1

date > "$marker"
echo "pronto: $imported itens importados, $characters personagens"
echo "detalhes: $log"
