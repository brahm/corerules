# Core Rules Community Add-ons Collection: relatório

O `addons.zip` (50.842.950 bytes) traz 686 pacotes: 492 `.xpx` da Expansion e 194 `.xpt` do Core Rules 2.0, contando a User Database e os dois arquivos que precisaram de conserto. Todos foram consolidados num único pacote, `Comunidade.xpx`, no formato que o `TSRP3.EXE` do CD da Expansion lê. Os 14 goodies mensais da Evermore Entertainment ficaram de fora depois, como explica a seção [Goodies da Evermore](#goodies-da-evermore): o pacote tem 36.525 itens de 672 addons.

O zip com o `Comunidade.xpx`, e como importá-lo no app com **Importar addons…**, estão no [GUIA.md](GUIA.md). O instalador AppImage para Linux, citado abaixo como foi testado, foi abandonado na versão 1.2.0 do app em favor do zip. Junto deste relatório:

| arquivo | conteúdo |
|---|---|
| `compatibilidade.csv` | uma linha por addon, com a situação, o número de itens e as observações |
| `conflitos.csv` | os nomes que aparecem em mais de um addon e qual versão ficou |
| `Comunidade-extras.xz` | as imagens e os textos dos addons, que vão no zip |

## Situação dos addons

| situação | addons |
|---|---|
| compatíveis sem mudança (os `.xpt` do 2.0 são convertidos na importação, como faz a própria Expansion) | 594 |
| convertidos: pontos de magia retirados das classes (89 addons e a User Database) | 90 |
| reparado (`pftf-umberlee.xpt`) | 1 |
| recuperado em parte (`Lots of Kits`) | 1 |
| inutilizável (`completebookofnecromancers.exe`) | 1 |

### Convertidos

Esses addons foram feitos com a Expansion depois dos patches 3 a 5, e o programa do CD recusa o pacote inteiro com "Unexpected file format.". As diferenças são duas:

* **Pontos de magia.** São objetos `CSpellPointsOb` dentro das classes. Afetam 293 classes, todas listadas em `compatibilidade.csv`, e a importação retira esses objetos. O resto da classe fica igual.
* **Kits na versão 31.** A versão 31 tem um byte a mais, sempre 0. Os 269 kits da User Database foram regravados na versão 30.

Teste no original: `barbarian.xpx` como veio é recusado e, convertido, é aceito. O `arvoreen.xpx`, cujas classes usam habilidades por nível (`CClassFreeSpellsOb`), já era aceito como veio.

### Reparados e recuperados

* **`Spells/pftf-umberlee/pftf-umberlee.xpt`** veio corrompido por uma transferência em modo texto, que trocou CR+LF por LF. Os tamanhos dos textos foram recalculados e o CR foi recolocado. O original recusa o arquivo como veio e lista os 11 itens do reparado.
* **`Kits/Lots of Kits.zip`** está truncado: tem 671.294 dos 1.045.873 bytes do `Kits.xpt`. Foram recuperados 103 kits inteiros, e o resto se perdeu.
* **`Dragon270.zip` e `Dragon272.zip`** também estão truncados. Os pacotes vieram inteiros, mas se perderam as imagens `Dragon Magazine 270 Portraits.zip`, `pic12.BMP`, `dragon 272 pics.zip` e `Dragonet, Pavilion Dragon.BMP`.
* **`Official Addons/completebookofnecromancers.exe`** é só o instalador web da InstallShield, sem o conteúdo, e não dá para aproveitar.
* **`CR2goodie9907/crbmputility.exe`** é um utilitário de imagens para Windows, não um addon.

### Fora do escopo, como combinado

* **Mapas do Map Maker II:** 30 mapas em 16 pacotes não foram migrados. Os pacotes estão marcados em `compatibilidade.csv`.
* **Desenhos do Campaign Mapper:** os 10 arquivos `.fcw` não foram migrados.

## Como foi consolidado

Os addons foram importados numa instalação limpa da Expansion, do mais antigo para o mais novo, pela data dentro do zip. A User Database (dezembro de 2002) entrou por último, por ser uma compilação que já inclui a maior parte dos outros. Depois, tudo foi exportado num só pacote.

Quando o mesmo nome aparece em mais de um addon, fica a versão mais nova, como faz o Import do original. São 11.863 nomes repetidos, e em 3.456 deles o conteúdo difere. A User Database cobre por completo 440 addons. Em `conflitos.csv` está, para cada nome, o addon que venceu e os outros onde ele aparece.

Algumas regras do Import do original se aplicam aqui:

* **Nomes de partes padrão.** Uma parte com o nome de uma parte padrão ganha um `*` no fim. Foi o caso de "Lesser Divination" e "Wild Magic" em `ghpb spheres-schools.xpt`.
* **Magias com o mesmo nome.** Magias de mago e de sacerdote dividem o mesmo espaço de nomes. "Freshen" existe nas duas listas da User Database, e ficou só a de sacerdote.
* **Raças sem combinações.** 36 raças da User Database não têm combinações raça/classe registradas (Aranea, os Avatares, Voadkyn, Xevis e outras). Elas entram sem classes permitidas, como no original.

## Goodies da Evermore

Os pacotes `cr2goodieAAMM` da pasta `Official Addons`, e o `cr2-march-goodie.zip` da pasta Equipment, são os goodies mensais que a Evermore Entertainment, a empresa que fez o Core Rules 2.0 para a TSR, publicou de 1998 a 2000. Não são da comunidade, e por isso saíram da coleção:

* **191 itens retirados:** 134 registros (a raça Phaser, os monstros Soul Seeker, Inner Torment e Skylene com os seus tipos, os encontros de The Black Rock e de Artheon, os equipamentos e itens mágicos) e 57 personagens (os 56 moradores de Artheon e Robert). A User Database tinha cópias de todos eles, e as cópias também saíram.
* **Cinco itens também estavam em `fotd.xpt` e `miscweaps.xpt`** (Utility belt, Water Pack, Fighting spurs, Hidden Dagger e Vial of (un)holy water/acid), com o texto da Evermore palavra por palavra, e saíram também.
* **Validação:** o pacote novo foi importado numa instalação limpa da Expansion, como o anterior. A primeira passagem deu os mesmos avisos e a segunda, nenhum. Os bancos do usuário diferem dos do pacote anterior só pelos itens retirados.
* Em `compatibilidade.csv` os 16 arquivos da Evermore estão marcados como fora da coleção. Em `conflitos.csv` os 134 nomes retirados têm "retirado da coleção" no lugar do vencedor.

Ficaram algumas menções de nomes nas listas de outros addons: kits de raças da comunidade citam "Phaser" e listas de armas citam "Hidden Dagger" e "Fighting spurs". São só nomes numa lista, como acontece com qualquer item que não existe na instalação.

## Validação

* **Leitura e regravação:** os 686 pacotes, o consolidado incluído, são lidos e regravados byte a byte pelo leitor do projeto.
* **Original da Expansion no Wine:** o diálogo Import do original abriu o `Comunidade.xpx` e listou os itens. A leitura levou cerca de 35 minutos. O Import All não foi executado no original.
* **Edição no app Linux:** depois da instalação, cada registro dos addons foi aberto, editado e salvo pelos mesmos editores que o app usa, e relido depois. Foram 10.993 magias, 173 armadilhas, 160 poderes psiônicos, 1.874 tipos de monstro, 5.315 monstros, 515 raças, 558 classes, 16.290 partes, 59 tesouros, 299 encontros e os 105 personagens importados. Nenhum falhou. Regravar os bancos sem mudanças mantém os arquivos idênticos byte a byte. Salvar um registro pelo editor faz os mesmos ajustes do original, como tirar espaços do fim do nome. Dez registros têm o nome de um registro padrão: seis classes ("Fighter", "Cleric", "Mage", "Illusionist", "Ranger" e "Thief"), três magias e uma raça. O app recusava salvá-los e agora aceita, desde que o nome continue o mesmo.
* **Instalador:** a AppImage foi testada numa cópia limpa do pacote de dados.
  * Importou 36.525 itens, 48 deles personagens, em menos de um minuto, e copiou 1.204 imagens.
  * Antes da retirada dos goodies da Evermore, eram 36.716 itens e 105 personagens.
  * A segunda passagem terminou sem nenhum aviso.
  * Os bancos do usuário ficaram idênticos byte a byte aos que o antigo script de instalação gravava.
  * Instalar de novo não duplica personagens nem altera os registros, a não ser por 4 combinações de raças cujo nome se repete no pacote.

## Compatibilidade com o app 1.0.0-rc1

O pacote foi conferido de novo com o app 1.0.0-rc1, que corrigiu o travamento ao importar o `Comunidade.xpx` pela janela do app (o bug crítico dos testes da 0.9.0).

* **Import pela janela do app:** a prévia abre e o Import traz os 36.525 itens sem travar, numa instalação limpa da Expansion.
* **Combinações raça/classe numa importação só.** Antes, uma combinação que cita duas classes novas do pacote só entrava numa segunda importação, como no original, e o Import do app mostrava 304 avisos "No race/class combinations could be added". Agora o Import fecha essas combinações na mesma vez. Sobram só os 36 avisos das raças que o pacote traz sem combinações, que são do próprio pacote. O instalador passou a importar uma vez só, e o resultado é idêntico byte a byte ao das duas passagens de antes.
* **Qilué Veladorn.** A personagem tem a classe "Sword Dancer", mas o pacote trazia a classe como "Sword Dancer " (com um espaço no fim, vinda de `sword dancer.xpx`). A ficha e o Edit Character recusavam abrir a personagem com "Sword Dancer is not a class in the databases". A classe e as suas 5 menções com espaço (2 combinações raça/classe e 3 listas de classes de kits e itens) passaram a "Sword Dancer", como já estavam nos outros 90 lugares do pacote. Nada mais mudou no pacote, o que foi conferido comparando os dois pacotes inteiros lidos pelo leitor do projeto.
* **Edição no app:** os 36.207 registros dos bancos do usuário foram abertos, passados campo a campo pelos editores da 1.0.0-rc1 e salvos, e nenhum foi recusado. Os 48 personagens abrem na ficha (texto, HTML e Markdown), são editados e salvos.
* **Instalador:** a AppImage 1.0.0-rc1 instalou o pacote numa cópia limpa em 22 segundos, com 36.525 itens, 48 personagens e 1.204 imagens, e gravou os mesmos bancos que o Import do app.

## Imagens e textos

Os addons trouxeram 685 imagens BMP e 321 textos. Sem os da Evermore, ficam 668 imagens e 309 textos, guardados em `Comunidade-extras.xz` e levados para o zip.

* **Para onde vai cada imagem.** Os registros guardam o caminho do Windows que a imagem tinha no computador do autor, como `C:\Program Files\AD&D Core Rules 2.0\Bitmaps\Monsters\Achaierai.BMP` ou `C:\AD&D Pics\troll.bmp`. Um caminho dentro de uma instalação do Core Rules, em qualquer drive, vira o mesmo caminho dentro da pasta de dados (`Bitmaps/Monsters/Achaierai.BMP`). Um caminho para o `Bitmaps` do CD fica em `Bitmaps`. Os outros vão para `UserBMP`, sem o drive (`UserBMP/AD&D Pics/troll.bmp`). No Wine, estes últimos também vão para o drive C:, onde o original os procura.
* **Escolha da imagem.** 528 imagens são citadas pelos registros e vão para 536 lugares. Quando dois addons trazem imagens com o mesmo nome, vale a do addon de onde veio o registro. Nenhum lugar recebe duas imagens diferentes. As 4 citações a imagens do próprio CD ficam com as do CD. Todas as 668 imagens também vão para `UserBMP/`, na pasta do seu addon, que é onde o diálogo de imagens do original abre.
* **Nada é sobrescrito.** Um arquivo que já existe fica como está, e o registro da instalação lista cada imagem gravada.
* **No app.** Tipos de monstro, monstros (pela imagem do seu tipo), raças e personagens mostram a imagem quando o arquivo está na pasta de dados. As imagens são lidas pelo próprio app, sem depender dos leitores de imagem do sistema. 3.800 citações apontam para imagens que nenhum addon trouxe, quase todas as do CD (`6.bmp`, `NA.bmp`), que **Extrair dados do CD…** copia do CD desde a versão 1.0.0-rc1.
* **Textos.** Os leia-me em texto, HTML, RTF, Word e PDF foram convertidos para texto simples. O zip leva esses textos em `texts/` e, em `addons.tsv`, o que cada pasta de addons trouxe para a coleção.
