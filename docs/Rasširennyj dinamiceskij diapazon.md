## Vvedenije

Mnogije staryje igry risovali kartinku v ocenj neboljšom diapazone jarkostej - v linejnom ili gamma-prorstranstve s 8 bitami na kanal.
Da i ne toljko staryje, nekotoryje igry eto delali jescö godu v 2015-m ("Wolfenstein: the Old Blood").

Odnako igry s rasširennym diapazonom jarkosti vyglädöt kuda lucše - "Half-Life 2 : Lost Coast", "The Elders Scrolls IV: Oblivion", "Crysis".
V etih igrah kartinka risujetsä v çvetovoj bufer s boljšim diapazonom i vysokoj tocnostju.
Obycno eto 16 bit na kanal (linejnyh) ili okolo togo.
Pered vyvodom kartinki na ekran proizvoditsä preobrazovanije etogo bufera - jarkostj privoditsä v nekij diapazon, ocenj casto nelinejnym sposobom.
Koeffiçient preobrazovanija (vyderžka) opredeläjetsä na osnove jarkostnyh harakteristik kadra.

## Realizaçija v programmnom rasterizatore

V programmnom rasterizatore tože možno (v teorii) risovatj kartinku v širokom diapazone jarkostej.

Samyj prostoj variant - podgotavlivatj poverhnosti v formate s boljšim diapazonom, risovatj ih naprämuju v bufer s boljšim diapazonom, posle cego provoditj preobrazovanije ekrannogo bufera.
Jestj podhody s boleje ekonomnym ispoljzovanijem pamäti.

Varianty podhodov:

### 16 bit na kanal poverhnosti, 16 bit na kanal itogovogo izobraženija

Itogo 64 bita (s vyravnivanijem) na pikselj poverhnosti i izobraženija.
Dostoinstvo - vsö hranitsä linejno, smešivanije budet prostym.
Nedostatok - potrebläjetsä mnogo pamäti, a dostup k nej ne bystr.

### "R11G11B10" dlä poverhnostej i izobraženija

Na kanal prihoditsä na 2-3 bita boljše, cem v 8-bitnom režime. Eto pozvoläjet rasširitj diapazon jarkostej, no ne siljno.
Dostoinstvo - smešivanije linejno, no trebujet cutj boljše operaçij (v sravnenii s 16 bit na kanal).
Nedostatok - cutj boleje složnyj kod po zapakovke poverhnostej i preobrazovaniju izobraženija v itogovoje.
Nedostatok - vozmožno diapazona budet ne dostatocno.

Možno ispoljzovatj dvuhstupencatoje preobrazovanije jarkosti.
Pri podgotovke poverhnostej možno linejno preobrazovyvatj çvet, ctoby (oçenocno) uložitjsä v nužnoje kolicestvo bit.
Potom možno proizvoditj okoncateljnoje preobrazovanije.

### RGB + masštab dlä poverhnostej i izobraženija

Na komponenty çveta otvodititsä po 8 bit, 8 bit otvoditsä na jarkostj (linejnuju ili stepeni dvojki).
Dostoinstvo - potreblenije pamäti po 32 bita na pikselj.
Dostoinstvo - ocenj širokij diapazon.
Nedostatok - složnaja pakovka, smešivanije budet nebystrym. Raspakovka pobystreje budet, no tože ne bystraja.
Nedostatok - poterä tocnostj tusklyh komponentov çveta, jesli jestj v razy boleje jarkije komponenty.

### 32 bita dlä poverhnostej i izobraženija, poverhnosti hranätsä s preobrazovanijem

Dostoinstvo - potreblenije pamäti po 32 bita na pikselj.
Dostoinstvo - ne nado provoditj preobrazovanije diapazona otdeljnym prohodom.
Nedostatok - iz-za nelinejnosti ne vozmožno praviljnoje smešivanije.