# Addons da comunidade para o Core Rules Expansion

O `addons.zip` (50.842.950 bytes) traz 686 pacotes: 492 `.xpx` da Expansion e 194 `.xpt` do Core Rules 2.0, contando a User Database e os dois arquivos que precisaram de conserto. Todos foram consolidados num único pacote, `Comunidade.xpx`, no formato que o `TSRP3.EXE` do CD da Expansion lê.

O instalador, que leva o `Comunidade.xpx` dentro dele, e como usá-lo estão no [README.md](README.md). Junto deste relatório:

| arquivo | conteúdo |
|---|---|
| `compatibilidade.csv` | uma linha por addon, com a situação, o número de itens e as observações |
| `conflitos.csv` | os nomes que aparecem em mais de um addon e qual versão ficou |

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

## Validação

* **Leitura e regravação:** os 686 pacotes, o consolidado incluído, são lidos e regravados byte a byte pelo leitor do projeto.
* **Original da Expansion no Wine:** o diálogo Import do original abriu o `Comunidade.xpx` e listou os itens. A leitura levou cerca de 35 minutos. O Import All não foi executado no original.
* **Edição no app Linux:** depois da instalação, cada registro dos addons foi aberto, editado e salvo pelos mesmos editores que o app usa, e relido depois. Foram 10.993 magias, 173 armadilhas, 160 poderes psiônicos, 1.874 tipos de monstro, 5.315 monstros, 515 raças, 558 classes, 16.290 partes, 59 tesouros, 299 encontros e os 105 personagens importados. Nenhum falhou. Regravar os bancos sem mudanças mantém os arquivos idênticos byte a byte. Salvar um registro pelo editor faz os mesmos ajustes do original, como tirar espaços do fim do nome. Dez registros têm o nome de um registro padrão: seis classes ("Fighter", "Cleric", "Mage", "Illusionist", "Ranger" e "Thief"), três magias e uma raça. O app recusava salvá-los e agora aceita, desde que o nome continue o mesmo.
* **Instalador:** a AppImage foi testada numa cópia limpa do pacote de dados.
  * Importou 36.716 itens, 105 deles personagens, em menos de um minuto.
  * A segunda passagem terminou sem nenhum aviso.
  * Os bancos do usuário ficaram idênticos byte a byte aos que o antigo script de instalação gravava.
  * Instalar de novo não duplica personagens nem altera os registros, a não ser por 4 combinações de raças cujo nome se repete no pacote.

## Imagens e documentação

As imagens BMP dos addons (retratos e figuras de monstros, que no original iam para `UserBMP/`) e os textos de documentação não estão no pacote: o app Linux não mostra essas figuras.
