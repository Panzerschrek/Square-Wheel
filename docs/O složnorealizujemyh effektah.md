Suscestvujet räd effektov, kotoryh v "SquareWheel" realizovatj vesjma problematicno.
Zdesj oni perecisleny i obosnovano, pocemu oni problematicny.


### Landšaft

Landšaft - boleje-meneje regulärnaja setka treugoljnikov, verojatno s "LOD"-ami.
Problema zaklücajetsä v tom, cto takaja setka ne ocenj družit s "BSP" derevom urovnä.

Poligony landšafta trebujut sortirovki s drugimi dinamiceskimi objektami.
Ctoby sortirovka rabotala boleje-meneje bystro, neobhodimo razbitj landšaft na množestvo "BSP" listjev, opredelitj, kakije treugoljniki landšafta vidny v nih i sortirovatj ih poštucno.
Pri etom voznikajet problema balansa - sliškom boljšoj razmer lista "BSP" dereva - dolgaja sortirovka, sliškom malenjkij razmer - izlišne mnogo listjev.
Takže nejasno, kak realizovatj sortirovku, kotoraja by korrektno rabotala dlä treugoljnikov, obrazujuscih sväznuju setku.

S osvescenijem landšafta vsö obstoit besjma neprosto.
Staticeski možno poscitatj svetokarty s ucötom gladkosti, hotj eto neprosto i nebystro.
Pri etom spekulär ne budet iz-za etoj gladkosti normaljno rabotatj, hotj eto i ne boljšaja problema dlä landšafta.

Boljšije problemy jestj s dinamiceskim osvescenijem.
Gladkim jego nikak ne sdelatj, a znacit, ono budet smotretjsä ne ocenj horošo.
Razve cto toljko svet ne budet scitatjsä poveršinno, no eto možet vyglädetj vesjma nekrasivo.

Landšaft možet potrebovatj svoih pravil naloženija svetokart.
Obycnyje svetokarty dlä nego sliškom detaljny.


### Krivolinejnyje poverhnosti

Primerno kak "Bazier Curves" v "Quake III Arena".

Problemy analogicny landšaftu - problemy s sortirovkoj, složno poscitatj svet, dinamiceskij svet ne budet gladkim.

Osobenno složno poscitatj svetokarty bez artefatov, ibo teksturnyje koordinaty scitajutsä vesjma hitro.

Naloženije dekalej možet bytj vesjma hitrym.


### Tuman

Jest raznyje varianty realizaçii tumana.

Pervyj variant - otrisovatj vsü sçenu v glubinu i na osnove glubiny poscitatj zatumanivanije i naložitj tuman na osnovnoje izobraženije.
Variant rabocij, no vesjma zatratnyj, a znacit redko primenimyj.
K tomu že tak možno realizovatj toljko globaljnyj tuman.
Drugoj minus dannogo podhoda - prozracnostj korrektno ne zatumanitsä.

Votorj variant - zapisyvatj zatumanivanije v poverhnosti.
Takoj variant pozvolit sdelatj tuman lokaljnym - naprimer, kotoryj jestj toljko v glubokih jamah, ili v opredelönnyh pomescenijah.
Podscöt zatumanivanija pri etom možno realizovatj poveržinno ili na ocenj gruboj setke v predelah poverhnosti.
Ctoby rascöt byl potocneje, kompilätor kart možet ottesselirovatj poverhnosti v objomah tumana.

Pri etom popoverhnostnyj rascöt tumana vsö jescö vesjma vycisliteljno-nakladen.
Pridötsä scitatj tuman otdeljnym prohodom nad poverhnostju, naprimer.

Takže nejasno, cto delatj s modelämi i sprajtami.
Dopustim, možno scitatj tuman konstantnym dlä modeli.
No rasterizaçija s zatumanivanijem vsö ravno vesjma nakladna.

Problema vseh podhodov - podobratj jarkostj tumana, horošo vyglädescego v "HDR" režime složnovato.

Tretij variant - možno realizovatj effekt, shožij s tumanom v sfericeskoj oblasti, prosto narisovav sprajt so smešivanijem.
Takoje možet rabotatj, no primenimo vesjma redko.


### Karty normalej dlä modelej

Karty normalej na geometrii urovnej rabotajut potomu, cto poligony urovnä ploskije, cto oznacajet, cto bazis teksturnyh koordinat možno poscitatj rovno odin raz dlä poligona.
S modelämi delo obstoit inace - osvescenije na nih gladkoje, a znacit, v predelah treugoljnika menäjetsä normalj (i potençialjno vesj ostaljnoj teksturnyj bazis).

K tomu že osvescenije na modeli nakladyvajetsä srazu pri rasteriazaçii.
Vypolnätj interpoläçiju bazisa teksturnyh koordinat, delatj vyborku iz karty normalej i scitatj svet na jejo osnove pri rasterizaçii - eto sliškom nakladno.

Posemu imejet smysl delatj nekij psevdoreljef, narisovannyj neposredstvenno v teksture modeli.


### Sistema castiç

Primerno kak v "Quake", kogda vsäkije effekty realizovyvalisj cerez množestvo castiç, nezavisimyh drug ot druga.
V "SquareWheel" s podobnym jestj problemy - castiçy nado kak-to sortirovatj dlä otrisovki, hotä by otnositeljno drugih objektov.
A eto dlä boljšogo kolicestva castiç vesjma zatratno.
V "Quake" eto ne bylo problemoj, t. k. tam byl bufer glubiny.

Aljternativa - realizaçija effektov gorazdo menjšim kolicestvom sprajtov, vozmožno animirovanyh, vozmožno daže proçedurno.