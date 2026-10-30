# Core Rules Community Add-ons Collection: guia de importação

Este pacote junta 672 addons feitos pela comunidade para o AD&D Core Rules 2.0 e a Expansion: raças, classes, kits, magias, monstros, equipamentos, encontros, módulos com personagens e a AD&D Core Rules User Database. Todos foram testados, convertidos para o formato da Expansion e consolidados num único arquivo, `Comunidade.xpx`, com 36.525 itens. As 668 imagens que vieram com eles também vêm junto. Os goodies mensais da Evermore Entertainment ficaram de fora. O [README](README.md) apresenta a coleção, em inglês, e o [relatorio.md](relatorio.md) conta o que foi convertido, reparado ou deixado de fora.

Mapas do Map Maker II e desenhos do Campaign Mapper não fazem parte do pacote, porque o app não trata desses programas.

O que é preciso:

* O app Core Rules 1.2.0 ou mais novo, no Linux, no Windows 11 ou no macOS, da [página de releases](https://github.com/brahm/corerules/releases) ou da Microsoft Store.
* Os dados da sua Expansion no app. Se ainda não tiver, abra o Core Rules e use **Extrair dados do CD…** no menu.

## Onde baixar

Os addons saem num zip, **[CoreRules-Community-Addons.zip](https://github.com/brahm/corerules/releases/download/addons/CoreRules-Community-Addons.zip)**, na [release addons](https://github.com/brahm/corerules/releases/tag/addons), com uma cópia de nome `CoreRules-Community-Addons-VERSÃO.zip` para cada versão. Ele serve para o comando **Importar add-ons…** do app, no Linux, no Windows e no macOS: escolha no menu a pasta onde está o zip, sem precisar extraí-lo, e o app lê todos os arquivos de uma vez, valida, importa os itens e copia as imagens para onde os registros as procuram, sem sobrescrever nenhum arquivo.

Dentro dele há uma pasta, `CoreRules-Community-Addons-VERSÃO`, com:

| Arquivo | Conteúdo |
| --- | --- |
| `Comunidade.xpx` | Os addons consolidados, no formato de exportação da Expansion; o Import do programa original também o lê |
| `pictures/` | As 668 imagens (BMP de 8 bits), nas pastas dos seus addons |
| `pictures.tsv` | Onde cada imagem vai na pasta de dados: a imagem, uma tabulação e o lugar |
| `texts/` | Os 309 textos dos autores, em texto simples |
| `addons.tsv` | O que cada pasta de addon trouxe para a coleção |
| `README.txt`, `LEIA-ME.txt` | Como instalar, em inglês e em português |

## Como importar

1. Baixe o `CoreRules-Community-Addons.zip` e ponha numa pasta só dele, por exemplo `Downloads/addons`. Não precisa extrair.
2. No Core Rules, escolha **Importar addons…**: no menu principal no Linux, no menu **Mais** no Windows e no menu **Arquivo** no Mac.
3. Escolha a pasta onde está o zip. O app procura nela, nas subpastas e dentro dos zips (até zips dentro de zips) todos os pacotes `.xpx` e `.xpt`, as imagens `.bmp` e o `pictures.tsv`.
4. Confira a lista: o que vai entrar, por categoria, o que foi achado duas vezes e o que não pode ser importado, com o motivo. Nada é gravado antes de você confirmar.
5. Clique em **Importar**. O `Comunidade.xpx` inteiro leva cerca de um minuto, com uma barra de progresso.
6. No fim, o relatório diz quantos itens, personagens e imagens entraram, e onde ficaram a cópia de segurança e o registro.

Os addons aparecem nos bancos de dados do usuário (Raças, Classes, Magias, Monstros e o resto) e os personagens dos módulos na lista de personagens. Tipos de monstro, monstros, raças e personagens mostram a imagem que veio com o addon. Os addons em si e os textos dos autores estão em inglês.

O mesmo comando serve para qualquer outra pasta de addons, soltos ou em zips, como a comunidade os compartilhou.

## O que a importação faz

1. Copia `UserDbas/` e `Chars/` para uma pasta ao lado da pasta de dados, por exemplo `data-before-addons-2026-10-29-232615/`.
2. Importa os pacotes do arquivo mais antigo para o mais novo, de modo que, num nome repetido, o mais novo vence, como no Import do programa original. Um registro do usuário com o mesmo nome de um addon é substituído pelo do addon. As classes passam uma segunda vez, para que uma combinação raça/classe que cita duas classes novas entre.
3. Deixa de fora os personagens que já estão na lista, então importar de novo não os duplica.
4. Copia as imagens para onde os registros as procuram, ou para o lugar que o `pictures.tsv` indica. Um arquivo que já existe não é substituído.
5. Grava um registro de tudo ao lado da pasta de dados, como `data-addons-2026-10-29-232615.log`.

Pacotes da Expansion não entram numa instalação do Core Rules 2.0, e arquivos danificados ou vazios são recusados; a lista antes da importação já mostra isso. O programa original do CD também importa o `Comunidade.xpx`, mas leva cerca de 35 minutos.

## Como desfazer

Troque `UserDbas/` e `Chars/` pela cópia de segurança, pondo no lugar de `DATA` a data da cópia que quer usar. As imagens gravadas estão listadas no registro da importação e podem ficar, já que nada as usa sem os addons:

```sh
cd ~/.local/share/corerules
rm -rf data/UserDbas data/Chars
cp -r data-before-addons-DATA/UserDbas data-before-addons-DATA/Chars data/
```

No Flatpak, a pasta é `~/.var/app/io.github.brahm.CoreRules/data/corerules`.

## Como o zip é gerado

O zip sai dos arquivos `addons/Comunidade.xpx.xz` e `addons/Comunidade-extras.xz` (imagens e textos) por `tools/addons_zip.py VERSÃO PASTA`, no workflow `Addons package` (`.github/workflows/addons.yml`), que o publica na release addons a cada mudança na main. O resultado é sempre o mesmo para a mesma entrada.
