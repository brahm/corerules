# Addons da comunidade para o Core Rules Expansion

Este pacote junta 686 addons feitos pela comunidade para o AD&D Core Rules 2.0 e a Expansion: raças, classes, kits, magias, monstros, equipamentos, encontros, módulos com personagens e a AD&D Core Rules User Database. Todos foram testados, convertidos para o formato da Expansion e consolidados num único arquivo, `Comunidade.xpx`, com 36.716 itens. O [relatorio.md](relatorio.md) conta o que foi convertido, reparado ou deixado de fora.

Mapas do Map Maker II e desenhos do Campaign Mapper não fazem parte do pacote, porque o app não trata desses programas.

## Onde baixar

O pacote é publicado no GitHub Packages do projeto, como [`corerules-addons`](https://github.com/brahm/corerules/pkgs/container/corerules-addons). É um artefato OCI, e não uma imagem de contêiner: o `docker pull` que a página do GitHub mostra não serve. Baixe com o [ORAS](https://oras.land):

```sh
oras pull ghcr.io/brahm/corerules-addons:latest
```

Ou só com `curl`:

```sh
repo=brahm/corerules-addons
token=$(curl -s "https://ghcr.io/token?scope=repository:$repo:pull" | sed 's/.*"token":"\([^"]*\)".*/\1/')
layer=$(curl -s -H "Authorization: Bearer $token" \
    -H "Accept: application/vnd.oci.image.manifest.v1+json" \
    "https://ghcr.io/v2/$repo/manifests/latest" |
    grep -o '"digest":"sha256:[0-9a-f]*"' | tail -n 1 | cut -d'"' -f4)
curl -L -H "Authorization: Bearer $token" -o addons-comunidade-expansion.tar.gz \
    "https://ghcr.io/v2/$repo/blobs/$layer"
```

Além de `latest`, cada versão do app tem a sua tag, como `0.7.2`.

## O que vem no pacote

`addons-comunidade-expansion.tar.gz` traz:

| arquivo | para que serve |
|---|---|
| `Comunidade.xpx` | os addons consolidados, no formato da Expansion |
| `instalar-addons.sh` | importa os addons no pacote de dados do app |
| `bin/cr` | a ferramenta de linha de comando que o instalador usa (binário estático, roda em qualquer Linux x86-64) |
| `extract-data.sh` | cria o pacote de dados a partir da sua instalação do Core Rules |
| `README.md`, `relatorio.md`, `compatibilidade.csv`, `conflitos.csv` | este guia e o relatório por addon |
| `SHA256SUMS` | as somas de verificação |

O aplicativo não vem no pacote. Instale o Core Rules 0.7.2 ou mais novo pela [página de releases](https://github.com/brahm/corerules/releases) (.deb, .rpm, Flatpak ou AppImage). Até a versão 0.7.1 o app não lia parte destes addons: classes com habilidades por nível (`CClassFreeSpellsOb`) e o que a Expansion com patch grava.

## O que é preciso

* Linux x86-64.
* O app Core Rules 0.7.2 ou mais novo.
* Uma instalação sua do AD&D Core Rules 2.0 Expansion, no Wine ou copiada do CD. Os addons não trazem as regras.

## Instalação

1. Abra o pacote:

   ```sh
   tar -xzf addons-comunidade-expansion.tar.gz
   cd addons-comunidade-expansion
   ```

2. Crie o pacote de dados do app a partir da sua instalação, se ainda não tiver um. Ele fica em `~/.local/share/corerules/data`.

   ```sh
   ./extract-data.sh --install "$HOME/.wine"        # Core Rules instalado no Wine
   ./extract-data.sh --install /caminho/do/Program  # ou a pasta que tem Database/
   ```

3. Instale os addons:

   ```sh
   ./instalar-addons.sh
   ```

   Para outra instalação da Expansion, informe a pasta: `./instalar-addons.sh /caminho/da/instalacao`.

4. Abra o app. Os addons aparecem nos bancos de dados do usuário (Raças, Classes, Magias, Monstros e o resto) e os personagens dos módulos na lista de personagens.

## O que o instalador faz

1. Confere se a pasta é uma instalação da Expansion.
2. Guarda `UserDbas/` e `Chars/` num `.tar.gz` ao lado da pasta, por exemplo `data-antes-dos-addons-20261017-230000.tar.gz`.
3. Importa o `Comunidade.xpx`. Um registro do usuário com o mesmo nome de um addon é substituído pelo do addon, como no Import do programa original.
4. Importa de novo só as classes. Uma combinação raça/classe que cita duas classes novas só entra quando as duas já existem.

Um registro do log fica ao lado da pasta. Os avisos "No race/class combinations could be added" da primeira passagem são esperados, porque a segunda passagem resolve todos eles.

Rodar o instalador de novo não duplica os personagens. Se a pasta estiver dentro de um prefixo do Wine, ele pede confirmação antes de mexer; o programa original do CD também lê o `Comunidade.xpx`, mas a leitura no original leva cerca de 35 minutos.

## Como desfazer

Apague `UserDbas/` e `Chars/` da pasta de dados e extraia a cópia de segurança no lugar:

```sh
cd ~/.local/share/corerules
rm -rf data/UserDbas data/Chars
tar -xzf data-antes-dos-addons-*.tar.gz -C data
```

## Como o pacote é gerado

`packaging/addons/build.sh` monta o pacote a partir de `addons/Comunidade.xpx.xz` e do `cr` compilado deste repositório. O workflow `Addons package` (`.github/workflows/addons.yml`) roda esse script e publica o resultado no GitHub Packages a cada mudança na main. O instalador fica em `packaging/addons/instalar-addons.sh`.
