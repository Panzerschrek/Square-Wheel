## Motivaçija

Dinamiceskoje osvescenije neobhodimo dlä boljšej interaktivnosti igrovogo mira.
Planirujetsä ispoljzovatj dinamiceskoje osvescenije dlä effektov vrode vspyšek vystrelov i vzryvov, snarädov (raket), fonarika u igroka, a takže dlä osobyh istocnikov osvescenija na karte.

Zadavatj dinamiceskoje osvescenije planirujetsä tocecnymi istocnikami.

Možno ispoljzovatj tenevyje karty (kubiceskije i proekçionnyje), no redko, ibo postrojenije tenevyh kart i vyborka iz nih vesjma nebystry.
V osnovnom planirujetsä ispoljzovatj istocniki sveta bez tenej.
Otsutstvije tenej prijemlimo dlä istocnikov sveta neboljšogo radiusa.


## Tehniceskije detali

Istocniki sveta možno razmescatj na karte tak že, kak i drugije dinamiceskije objekty, sopostavläja listjam "BSP" dereva istocniki sveta i naoborot.
Dlä etogo pridötsä zadavatj istocnikam sveta radius, ctoby oni imeli konecnyje razmery.
A ctoby ne bylo artefaktov na graniçe radiusa istocnika osvescenija, nado podkorrektirovatj formulu vycislenija intensivnosti ot rasstojanija, ctoby na rasstojanii boljše radiusa intensivnostj byla nulevoj.

Vozmožno, stoit kak-to ogranicitj kolicestvo istocnikov sveta dlä každogo poligona i dlä vsej sçeny, ctoby v složnyh slucajah ne ronätj proizvoditeljnostj.


### Poverhnosti

Dlä každogo poligona sledujet ucityvatj toljko te istocniki osvescenija, cto nahodädsä v jego BSP liste.
Krome etogo stoit otbrasyvatj istocniki sveta za poligonom i istocniki, na rasstojanii ot poligona boljšem sobstvennogo radiusa.

Ne stoit dlä istonikov sveta bez tenej otbrasyvatj poligony na osnovanii vidimosti po "BSP" derevu, ibo eto budet sozdavatj (inogda) diskretnuju tenj (po poligonam).

Vycislätj osvescenije dlä poverhnostej planirujetsä potekseljno, daby poscitatj osvescenije s ucötom normali každogo tekselä.

Možno konecno poprobovatj realizovatj dinamiceskoje osvescenie cerez modifikaçiju svetokart ili sozdanije otdeljnyh svetokart.
No takoj podhod trebujet složnogo vycislenija napravlennyh svetokart, cto k tomu že dajot problemy s ucötom množestva istocnikov sveta i spekulära ot nih.
K tomuže podhod na osnove svetokart ne dajot polucit rezkije teni.


### Tenevyje karty

Tenevyje karty nužno stroitj toljko dlä istocnikov sveta, dlä kotoryh vklüceno ispoljzovanije tenevoj karty.
Stroitj tenevyje karty možno paralleljno - dlä každoj proekçionnoj tenevoj karty i dlä každoj storony kubiceskoj tenevoj karty.

Vstrojennyje modeli stoit ucityvatj v tenevoj karte, daby ne vydavatj otsutstvijem tenej sekretnyje dveri.

Modeli iz treugoljnikov mogut otbrasyvatj teni.
No dlä etogo modeli nado pereanimirovatj, ctoby sproeçiroatj treugoljniki v konkretnuju tenevuju kartu.
Da i rasterizaçija množestva melkih treugoljnikov - delo nebystroje.
Poetomu stoit sdelatj otbrasyvanije tenej modelämi otklücajemym.

Razmer tenevoj karty stoit vybiratj na osnovanii poziçii istocnika sveta otnositeljno kamery.
Dlä slucajev s boljšim kolicestvom tenevyh kart stoit rassmotretj variant so sniženijem ih razrešenija.


### Osvescenije modelej iz treugoljnikov

Modeli iz treugoljniov neobhodimo osvescatj dinamiceskimi istocnikami sveta.
Pri etom osvescenije každoj veršiny otdeljno kažetsä rastociteljnym.
Vmesto etogo stoit osvescatj vsü modelj primerno tak že, kak i staticeskim osvescenijem - s pomoscju kuba osvescenija i (vozmožno) vektorom preimuscestvennogo napravlenija sveta.

Parametry osvescenija modeli stoit scitatj na osnove jejo ogranicivajuscego parallelepipeda, a ne prosto scitatj osvescenije dlä çentra.
Eto neobhodimo, ctoby modelj ne stanovilosj beskonecno-jarkoj, jesli jejo çentr sovpadajet s poziçijej istocnika sveta.


### Osvescenije dekalej

Dekali možno osvescatj primerno tak že, kak i modeli iz treugoljnikov - vycislätj odno znacenije sveta (kub) na vsü dekalj i dlä každogo poligona, kotoryj etoj dekalju zatragivajetsä, ispoljzovatj normalj dlä polucenija znacenija sveta.
Aljternativnyj variant - scitatj osvescenije poveršinno.
No eto možet bytj dorogo.