### Opyt predšestvennikov

#### Pervoje pokolenije

Igry tipa "Doom" i "Duke Nukem 3d" riosvali stolbçy/stroki pikselej s postojannym osvescenijem naprämuju.
Osvescenije primenälosj cerez vyborku iz tabliçy.
Dlä každogo urovnä jarkosti (64 gradaçii) svoja tabliça preobrazovanija iz ishodonogo çveta v zatenönnyj.
Çveta byli palitrovymi i tabliçy bili neboljšimi.

Steny risovalisj po stolbçam.
Po stolbçam že hranilisj tekstury sten.
Eto delalo kod teksturirovanija vesjma prostym i ot etogo bystrym.

Poly risovalisj cutj složneje.
Na nih byli tajläscijesä tekstury s razmerom stepeni dvojki.
Pri prohoždenii stroki pola/potolka izmenälisj obe teksturnyje koordinaty. Pered vyborkoj koordinaty privodilisj v diapazon (dve operaçii "&").

V podhode so strogo-gorizontaljnymi polami i strogo-verticaljnymi stenami "Z" koordinata ne menäjetsä vdolj stolbça/stroki, cto pozvoläjet vycislätj 1 / "Z" odin raz, a ne popikseljno, ctoby dobitjsä perspektivno-korrektnogo teksturiovanija.

Uporädocivanije poverhnostej po glubine delalosj cerez otsecenije v ekrannom prostrabstve.
V "Doom" bylo risovanije poverhnostej ot bližnih k daljnim s podderžanijem dvuhmernyh graniç narisovannogo, ctoby otsekatj risujemoje.
V "Duke Nukem 3D" bylo necto shožeje, no tam byli sektora i portaly.

#### Vtoroje pokolenije

Igry tipa "Quake" i "Unreal" risovali polnostju tröhmernyje urovni s vozmožnostju krutitj golovoj.
Poverhosti byli pokryty svetokartami.
V etih igrah risovanije razdelilosj na dva etapa.
Pervyj etap - sozdatj nekije vremennyje tekstury dlä poverhnostej, kombinirujuscije svetokartu i sobstvenno teksturu.
Vtoroj etap - rasterizaçija polucennij na pervom etape tekstury.

Pri etom na pervom etape svetokarta linejno interpolirujetsä.
Delatj takoje na etape rasterizaçii sliškom resursozatratno.
Dopolniteljnyj profit ot generaçii vremennyh tekstur - ih keširovanije meždu kadrami.

V "Half-Life" dodumalisj k teksturam poverhnostej dobavlätj dekali.
V "Unreal" cerez pohožij mehanizm realizovyvali approksimaçiju objemnogo tumana.

Rasterizaçija poverhnostej teperj eto polnoçennaja rasterizaçija poligonov.
Pri etom provoditsä approksimaçija vycislenija 1 / "Z" dlä perspektivnoj korrekçii, kusocno-linejnym sposobom, k primeru.

V "Quake" uporädocivanije poverhnostej bylo po "BSP" derevu (s pererisovyvanijem). Izlišneje pererisovyvanije ustranälosj cerez "PVS".
V "Unreal" HZ kak bylo.

Modeli v "Quake" prohodili popikseljnyj test glubiny.
V "Unreal" byl kakoj-to boleje hitryj test na osnove bufera glubiny, predstavlenno kusocno-linejnym sposobom.
V "Thief" modeli narezalisj po sektoram urovnä, poligony sortirovalisj.

Çvet byl 8-bitnym v "Quake", pozže v "Half-Life" sdelali podderžku 16-bitnogo çveta, a v "Unreal" - 32-bitnogo.

#### Unreal Tournament 2004

V nöm jestj režim programmnogo rasterizatora. Informaçii po nemu poka netu.

#### Jurrastic Park: Trespasser

Nado vyjasnitj, kak on rabotal.

#### Taras Buljba 3D

Portaljno-sektornyj tröhmernyj dvižok.
Rabotajet vsö sverhbystro, ibo geometrija topornaja, çvet 8-bitnyj, bufera glubiny i testov nikakih netu.

#### PanzerChasm

Poverhnosti keširovalisj, kak v "Quake".
Uporädocivanije sten bylo po "BSP" derevu ot bližnih k daljnim.
Dlä ustronenija pererisovki ispoljzovalsä odnobidnyj bufer perekrytija s uskorenijem testa cerez ijerarhicnostj.
Takže popolnälsä bufer glubiny.

Modeli risovalisj s testom glubiny (ijerarhiceskim, otcasti).

Steny rasterizovalisj s kusocno-linejnoj approksimaçijej 1 / "Z". Na polah 1 / "Z" scitalsä odin raz na liniju, ibo poly vsegda nakloneny k kamere osobym obrazom.

Çvet byl 32-bitnym, bylo cestnoje smešivanije.
Byl aljfa-test.


Kak mne kažetsä, risovatelj v "PanzerChasm" byl ne samym bystrym iz vozmožnyh, ibo byl pereusložnönnym, pisal až v tri mesta, stradal ot pererisovki.
