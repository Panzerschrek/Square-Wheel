## Zadumka

Hocetsä sozdatj graficeskij dvižok s programmnym rasterizatorom, rabotajuscim na çentraljnom proçessore, prevoshodäscij takovyje iz staryh igr ("Quake", "Unreal", "Duke Nukem 3D" i drugije) po graficeskim effektam.
Vo vseh etih igrah maksimum, cto bylo, eto staticeski osvescönnyje poverhnostri, s dobavlenijem (inogda) prostogo dinamiceskogo osvescenija.
Ja že hocu poprobavatj realizovatj sledujuceje:
* Dinamiceskoje osvescenije s dinamiceskimi že tenämi (kak bylo, nacinaja s "Doom 3").
* Ispoljzovanije kart normalej.
* Ispoljzovanije spekulärnyh otraženij.
* (vozmožno) risovanije v rasširennom dinamiceskom diapazone ("Half-Life: Lost Coast" i pozže).

Obobscaja - hocetsä kacestvennogo proryva. Togo, cto bylo uže sozdano v epohu dominirovanija videokart.

Važnyj aspekt - pri etom dvižok dolžen podhoditj dlä sozdanija dinamicnyh igr šuterov), cto oznacajet neobhodimostj raboty s castotoj kadrov 60 Gerç i vyše s prijemlimym kacestvom.

### O tom, kak vyžatj maksimum proizvoditeljnosti

Ctoby dobitjsä trebujemyh effektov, nužno vyžatj maksimum resursov iz çentraljnogo proçessora.

Sovremennyje proçessory superskalärny - umejut vypolnätj neskoljko instrukçij nezavisimo.
Poetomu stoit planirovatj arhitekturu tak, ctoby vycislenija v ramkah odnogo potoka upravlenija mogli bytj rasparaleleny, cto oznacajet otsutstvije sväzi castej vycislenij po dannym.
Takže važno, ctoby ne proishodilo sbrosa konvejera vycislenij v proçessore iz-za neverno predksazannyh vetvlenij.
Dlä etogo nado, ctoby vetvlenij ili voobsce ne bylo, ili že ctoby oni byli predskazujemy po kakomu-to priznaku.

Takže ne stoi zabyvatj, cto na sovremennyh proçessorah dostup k pamäti, osobenno neposledovateljnyj, tože vesjma dorog.
Poetomu stoit minimizirovatj ctenija/zapisi i po vozmožnosti ih lokalizovatj.

Stoit rassmotretj vozmožnostj ispoljzovanija vektornyh instrukçij proçessora ("SSE" i procije).
Vožmožno, ih ispoljzovanije možet pomocj v uskorenii koda.

V poslednüju oceredj stoit rassmotretj vozmožnostj rasparallelivanija vycislenij meždu potokami na raznyh jadrah proçessora.
Eto možet kratno povysitj proizvoditeljnostj, v nekotoryh slucajah.
No, važno pri etom ne konvejerizirovatj vycislenija meždu potokami, ctoby ne vyzyvatj zaderžek v risovanii kadra.

### Çelevoje oboudovanije i ožidajemyj rezuljtat

Dvižok dolžen rabotatj na proçessorah "Intel" (i5), nacinaja gde-to s "Kaby Lake" i "AMD", nacinaja s "Ryzen" na castote ot 3Ggç.
Bylo by neploho takže rabotatj na aljternativnyh arhitekturah (izolirovatj arhitekturno-zavisimyj kod).
Jestestvenno, sledujet zakladyvatjsä na 64-bitnyj proçessor s nativnoj podderžnoj operaçij nad 64-bitnymi çelymi cislamu, v tom cisle umnoženijem i delenijem.

Dostatocnym rezuljtatom budet stabiljnaja rabota dvižka (60+ Gç) v razrešenii 960x540 s posledujuscim rastägivanijem do 1920x180 ("Full HD").
Želateljnym rezuljtatom budet stabiljnaja rabota v rasterizaçijej v "Full HD".
