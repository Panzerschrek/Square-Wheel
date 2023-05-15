Vo mnogih dvižkah suscestvujut takije effekty, kak portaly, zerkala, kamery i t. d.
Obsceje u vseh etih effektov to, cto dlä nih nado risovatj sçenu dopolniteljno - iz drugoj tocki obzora.

V _SquareWheel_ hotelosj by takije effekty realizovatj.

### Opredelenija

* Zerkalo - ploskaja poverhnostj, otražajuscaja luci pod trem že uglom, pod kotorym oni na nejo padajut. Tocka obzora - otraženije tocki kamery otnositeljno ploskosti zerkala.
* Portal - para ploskih poligonov, cerez odin vidno to, cto nahoditsä za drugim. Pri etom izmenenije tocki obzora vyzyvajet parallaks izobraženija, vidimogo cerez poral.
* Kamera i monitor. Izobraženije risujetsä iz fiksirovannoj tocki v odnom meste i otobražajetsä na nekotorom poligone v drugom.


### Podhod s prämym risovanijem

Zerkala casto risujut sledujuscim obrazom:
strojat speçialjnuju matriçu dlä izobraženija v zerkale takim obrazom, ctoby otrisovannoje risovalosj na tot že ekran.
Pri etom nado vo-pervyh kak-to otsecj to, cto nehoditsä vne ekrannyh graniç zerkala.
Na "GPU" dlä etogo ispoljzujut "Stencil".
Vo-vtoryh nado otsecj i ne risovatj to, cno nahoditsä za zerkalom.
Na "GPU" dlä etogo ispoljzujetsä poljzovateljskaja ploskostj otsecenija.

Analogicnym obrazom realizujutsä takže poraly.

Dostoinstov dannogo podhoda - izobraženije v zerkale pikselj v pikselj sovpadajet s ekrannym buferom.
Ono stolj že cötko.
Takim obrazom možno daže realizovatj besšovnyje portaly, kak v "Portal" ili v "Antichamber".
Pri etom inogda možno risovatj izobraženije prämo na ekran.
Pri etom pri renderinge na "GPU" možno umenjšitj bufer izobraženija, ctoby sekonomitj resursy.

Možno poverh izobraženija cto-to narisovatj (promodulirovatj) - steklo ili jescö cto-to podobnoje.

Nedostatki - nužno otsekatj izobraženije v ekrannom i mirovom prostranstve.
Pri prämom risovanii na ekran nevozmožno sdelatj poluprozracnoje zerkalo (poverhnostj vody).

Primeniteljno k _SquareWheel_ takoj podhod imejet sledujuscije nedostatki:
* Nužno otsekatj vsü risujemuju geometriju po proizvoljnomu ekrannomu poligonu. Eto nebystro v sravnenii s fiksirovannymi vosjmiugoljnikami, ispoljzujuscimisä sejcas.
* Dlä zerkal trebujetsä modifikaçija koda dlä vozmožnosti risovanija poligonov s obratnym porädkom veršin (po casovoj strelke/protiv casovoj strelki).
* Ne rabotajet poluprozracnostj.


### Podhod s risovanijem v teksturu

Aljternativnyj podhod - risovatj izobraženije v teksturu.
Pri etom tekstura nakladyvajetsä na poligon obycnym obrazom (kak lübaja drugaja), iz-za cego jejo tekseli ne sovpadajut s pikselämi ekrana.

Dostoinstva dannogo podhoda:
* Možno risovatj v menjšem razrešenii.
* Ne trebujetsä otsekatj izobraženije vne ekrannyh graniç.
* Legko realizujetsä poluprozracnostj.

Nedostatki:
* Izobraženije necötko.
* Trebujetsä dopolniteljnaja pamätj pod teksturu.
* Nužno perekladyvatj izobraženije iz tekstury na ekran - lišnije operaçii s pamätju.

Primeniteljno k _SquareWheel_ jestj sledujuscije nüansy:
* Dlä zerkal ne nado realizovyvatj risovanije otražönnyh poligonov - vmesto etogo možno otrazitj samu teksturu.
* Ploskostj otsecenija zerkala ili porala možet sovpadatj s bližnej ploskostju otsecenija.

Krome togo takoj podhod obladajet nekoj hudožestvennoj çennostju - izobraženije stolj že piksilizirovano, kak i tekstury.


### Detali realizaçii v _SquareWheel_

Pri lübom podhode v _SquareWheel_ nužno budet vnesti sledujuscije dorabotki:
* Räd vnutrennih struktur, hranäscih sostojanije kadra otnositeljno tocki obzora, neobhodimo budet produblirovatj.
  K nim otnosätsä informaçija o listjah, poligonah, modeläh i t. d.
  Ne vpolne jasno, otnositsä li k nim že bufer poverhnostej.
  Vozmožno, možno obojtisj odnim, jesli podgotavlivatj zerkala/poraly/kamery pered drugimi poverhnostämi.
* Dublirovanija možet ne hvatitj.
  Togda želateljno voobsce, sozdatj rekursivnuju posledovateljnostj etih struktur (s ogranicenijem).
  Eto neobhodimo dlä risovanija portalov cerez portaly, zerkal v zerkalah i t. d.
* Neobhodimo dorabotatj kod vycislenija vidimosti uzlov - ctoby rekursivnyj poisk možno bylo nacinatj s listjev, gde nahoditsä vyhodnoj portal/zerkalo, a ne gde nahoditsä kamera.
* Nado kak-to dorabotatj sistemu materialov, daby ukazyvatj materialam portalov/zerkalov, kak i cto poverh nih risovatj.
* Nužen kontrolj so storony igrovogo koda - ctoby portaly/zerkala možno bylo vklücatj, vyklücatj, peremescatj i t. d.
* Dlä kamer želateljno bylo by imetj neskoljko displejev.

Boleje perspektivnym v _SquareWheel_ kažetsä podhod s risovanijem v teksturu.
Važno eto tem, cto takoj podhod ne trebujet suscestvenno usložnätj osnovnoj kod risovanija, cto možet jego zamedlitj.
