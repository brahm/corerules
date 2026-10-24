# Core Rules Community Add-ons Collection: guia de instalação

Este pacote junta 672 addons feitos pela comunidade para o AD&D Core Rules 2.0 e a Expansion: raças, classes, kits, magias, monstros, equipamentos, encontros, módulos com personagens e a AD&D Core Rules User Database. Todos foram testados, convertidos para o formato da Expansion e consolidados num único arquivo, `Comunidade.xpx`, com 36.525 itens. As 668 imagens que vieram com eles também são instaladas. Os goodies mensais da Evermore Entertainment ficaram de fora. O [README](README.md) apresenta a coleção, em inglês, e o [relatorio.md](relatorio.md) conta o que foi convertido, reparado ou deixado de fora.

Mapas do Map Maker II e desenhos do Campaign Mapper não fazem parte do pacote, porque o app não trata desses programas.

## O instalador

Os addons são distribuídos num único arquivo, `CoreRules-Community-Addons-VERSÃO-x86_64.AppImage`. Ele é um programa com janela que leva os addons dentro dele e os instala no Core Rules, sem script nem outros arquivos.

O que é preciso:

* Linux x86-64 com glibc 2.43 ou mais nova (Fedora 44, Ubuntu 26.04 ou mais novos).
* O app Core Rules 0.7.2 ou mais novo, da [página de releases](https://github.com/brahm/corerules/releases). Até a versão 0.7.1 o app não lia parte destes addons: classes com habilidades por nível (`CClassFreeSpellsOb`) e o que a Expansion com patch grava.
* Os dados da sua Expansion no app. Se ainda não tiver, abra o Core Rules e use **Extrair dados do CD…** no menu.

## Onde baixar

**[Baixe o instalador aqui](https://github.com/brahm/corerules/releases/download/addons/CoreRules-Community-Addons-x86_64.AppImage).** O link sempre traz a versão mais nova. A [release addons](https://github.com/brahm/corerules/releases/tag/addons) também guarda cada versão.

O mesmo arquivo é publicado no [GitHub Packages](https://github.com/brahm/corerules/packages) do projeto, de duas formas, porque o GitHub Packages não tem um registro para arquivos soltos:

* **Como o arquivo AppImage**, no pacote Maven [`io.github.brahm.corerules-community-addons`](https://github.com/brahm/corerules/packages/3339900). A página dele lista `corerules-community-addons-VERSÃO-x86_64.AppImage` em Assets, para baixar pelo navegador. Baixar desse registro pela linha de comando pede um token do GitHub, mesmo com o pacote público.
* **Como artefato OCI**, [`corerules-community-addons`](https://github.com/brahm/corerules/pkgs/container/corerules-community-addons), que baixa sem conta nenhuma. Ele guarda a AppImage como seu único arquivo, e não uma imagem de contêiner: o `docker pull` que a página do GitHub mostra não serve. Para baixar de lá, use o [ORAS](https://oras.land):

```sh
oras pull ghcr.io/brahm/corerules-community-addons:latest
```

Ou só com `curl`:

```sh
repo=brahm/corerules-community-addons
token=$(curl -s "https://ghcr.io/token?scope=repository:$repo:pull" | sed 's/.*"token":"\([^"]*\)".*/\1/')
layer=$(curl -s -H "Authorization: Bearer $token" \
    -H "Accept: application/vnd.oci.image.manifest.v1+json" \
    "https://ghcr.io/v2/$repo/manifests/latest" |
    grep -o '"digest":"sha256:[0-9a-f]*"' | tail -n 1 | cut -d'"' -f4)
curl -L -H "Authorization: Bearer $token" -o CoreRules-Community-Addons-x86_64.AppImage \
    "https://ghcr.io/v2/$repo/blobs/$layer"
```

Além de `latest`, cada versão do app tem a sua tag, como `1.0.0`.

## Instalação

1. Torne o arquivo executável e abra-o:

   ```sh
   chmod +x CoreRules-Community-Addons*x86_64.AppImage
   ./CoreRules-Community-Addons*x86_64.AppImage
   ```

   Sem FUSE, rode com `APPIMAGE_EXTRACT_AND_RUN=1`.

2. A janela procura o Core Rules sozinha e mostra cada cópia que achou, com a pasta de dados que ela lê:

   | Como o app foi instalado | Pasta de dados |
   | --- | --- |
   | Flatpak | `~/.var/app/io.github.brahm.CoreRules/data/corerules/data` |
   | Pacote .deb ou .rpm | `~/.local/share/corerules/data` |
   | AppImage (em Downloads, Aplicativos, Área de Trabalho, `~/.local/bin`, `/opt` ou a pasta pessoal) | `~/.local/share/corerules/data` |

   O botão **Detalhes dos addons…** mostra, quando você quiser, cada pasta de addons com o que ela trouxe para a coleção e os textos que os autores escreveram.

   Sem essa pasta, cada cópia usa a instalação do programa original no Wine, em `~/.wine/drive_c/Program Files/AD&D Core Rules 2.0`, como o próprio app faz. Se a cópia ainda não tem os dados do jogo, o botão **Abrir o Core Rules** a abre para você usar **Extrair dados do CD…**; depois clique em ↻ para procurar de novo. Para instalar numa pasta que o instalador não achou, use **Outra pasta…** e indique a pasta que tem `Database`.

3. Clique em **Instalar**. A importação leva menos de um minuto.

4. Abra o Core Rules, pelo botão **Abrir o Core Rules** ou como de costume. Os addons aparecem nos bancos de dados do usuário (Raças, Classes, Magias, Monstros e o resto) e os personagens dos módulos na lista de personagens. Tipos de monstro, monstros, raças e personagens mostram a imagem que veio com o addon. Se o app já estava aberto, feche e abra de novo.

A janela fala os mesmos idiomas do app (português, inglês, espanhol, italiano, francês e alemão) e segue o que foi escolhido nas Preferências do Core Rules; sem escolha, segue o idioma do sistema. Os addons em si e os textos dos autores estão em inglês.

## O que o instalador faz

1. Confere se cada pasta é uma instalação da Expansion. Quando duas cópias do app leem a mesma pasta, ela recebe os addons uma vez só.
2. Copia `UserDbas/` e `Chars/` para uma pasta ao lado, por exemplo `data-before-addons-20261019-010700/`.
3. Importa o `Comunidade.xpx`. Um registro do usuário com o mesmo nome de um addon é substituído pelo do addon, como no Import do programa original.
4. Importa de novo só as classes. Uma combinação raça/classe que cita duas classes novas só entra quando as duas já existem.
5. Copia as imagens para onde os registros as procuram (`Bitmaps/Monsters`, `Bitmaps/Portrait` e `UserBMP`, e no Wine também as pastas do drive C: que os registros citam). Um arquivo que já existe não é substituído.

O registro da importação fica ao lado da pasta, como `data-addons-20261019-010700.log`, e o botão **Abrir o registro** o mostra no fim.

Instalar de novo não duplica os personagens. Se a pasta estiver dentro de um prefixo do Wine, o instalador pede confirmação antes de mexer. O programa original do CD também lê os addons, mas o Import dele leva cerca de 35 minutos.

## Como desfazer

Troque `UserDbas/` e `Chars/` pela cópia de segurança, pondo no lugar de `DATA` a data da cópia que quer usar. As imagens gravadas estão listadas no registro da instalação e podem ficar, já que nada as usa sem os addons:

```sh
cd ~/.local/share/corerules
rm -rf data/UserDbas data/Chars
cp -r data-before-addons-DATA/UserDbas data-before-addons-DATA/Chars data/
```

No Flatpak, a pasta é `~/.var/app/io.github.brahm.CoreRules/data/corerules`.

## Como o instalador é gerado

O instalador é o crate `crates/cr-addons`, em Rust com GTK 4 e libadwaita. Ele embute `addons/Comunidade.xpx.xz` e `addons/Comunidade-extras.xz` (imagens e textos) e importa pelo mesmo código do app (`cr-store`). `packaging/appimage/build-in-sdk.sh VERSÃO addons` gera a AppImage no SDK do GNOME 51, e o workflow `Addons package` (`.github/workflows/addons.yml`) a publica no GitHub Packages a cada mudança na main.
