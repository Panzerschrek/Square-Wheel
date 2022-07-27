Dlä risovanija neba çelesoobrazno bylo by realizovatj nebesnyj kub.
Eto ne jedinstvennyj sposob realizaçii neba, no vesjma rasprostranönnyj i dajuscij otnositeljno horošij rezuljtat.

Zadavatj nebo stoit cerez speçialjnyj flag v materiale.
Poligony s materialom neba možno ne tesselirovatj pri postrojenii karty i ne osvescatj pri postrojenii svetokart.

Gruzitj tekstury neba možno iz izobraženij (obycnyh ili "HDR").
Pri etom možno vse storony upakovyvatj v odno izobraženije

Hranitj v dvižke tekstury neba çelesoobrazno srazu v "HDR" formate (16 bit na komponent), ctoby naprämuju ih rasterizovatj.
No dlä 32-bitnogo režima tože nužno hranitj variant tekstyry v 32-bitnom formate.


### Risovanije - variant 1

Dlä poligonov neba ne nužno stroitj poverhnosti.
Pri rasterizaçii poligonov neba nado vmesto rasterizaçii etogo poligona delatj rasterizaçiju poligonov nebesnogo kuba s obrezkoj ih po graniçam tekuscego poligona.

Takoj podhod legko integrirujetsä v suscestvujuscij proçess risovanija urovnä.
On podderživajet poligony neba v proizvoljnyh mestah, raznyje nebesa v odnom kadre.
Ploscadj rasterizaçii ravna ploscadi vidimyh poligonov neba.

Nedostatok - mnogo obrezok polignov neba, nekotoraja nelokaljnostj rasterizaçii, kogda nebo pobito na množestvo poligonov.


### Risovanije - variant 2

Poligony neba ne risujutsä vovse.
Vmesto risovanija poligonov neba prosto stroitsä ogranicivajuscij vosjmiugoljnik vokrug vseh vidimyh poligonov neba.
Daleje, pered rasterizaçijej urovnä rasterizujutsä poligony nebesnogo kuba s obrezkoj po polucennomu raneje vosjmiugoljniku.

Dostoinstva:
Takoj podhod obladajet boljšej lokaljnostju pri rasterizaçii.
Kod risovanija poligonov urovnä pocti nikak ne menäjetsä.
Vozmožno rasširenije podhoda - krome samogo kuba možno risovatj inuju geometriju dlä solnça/luny, oblakov, landšafta, zdanij.

Nedostatki:
V nekotoryh slucajah ploscadj rasterizaçii možet okazatjsä siljno boljše neobhodimoj.
Ne vozmožno risovatj bojše odnogo neba v odnom kadre.
Ne vozmožno narisovatj poligony urovnä za nebom.