Horošo by imetj v dvižke razlicnyje teksturnyje effekty, ctoby s pomoscuju nih možno bylo realizovatj vsäkije krasivosti.

### Skoljženje tekstur (realizovano)

Samyj prostoj effekt.
K teksturnym koordinatam prosto dobavläjetsä smescenije, linejno zavisimoje ot vremeni.


### "Turb" effekt (realizovano)

Tekstura iskažajetsä sinusoidaljnymi volnami.
Tak delalosj, naprimer, s vodoju v "Quake".
Realizovan effekt cerez zapisj modifiçirovannoj tekstury v otdeljnyj bufer.


### Izlucajuscij sloj (realizovano)

Dlä nekotoryh effektov možno ukazatj teksturu izlucajuscego sloja.
Eta tekstura prosto primenäjetsä k poverhnosti posle jejo postrojenija i umnožajetsä na zadannyj v svojstvah materiala svet.


### Pokadrovaja animaçija (realizovano)

Tekstura çikliceski menäjetsä vo vremeni s ispoljzovanijem räda izobraženij.
Eto ne trebujet dopolniteljnyh vycislenij - prosto každyj kadr menäjetsä indeks v tabliçe tekstur na tekuscij kadr animaçii.
Realizovano eto kak sväzannyj spisok materialov.
Cerez zadanije razlicnogo nacaljnogo materiala možno realizovatj razlicnuju nacaljnuju fazu animaçii u razlicnyh poligonov.


### Smena tekstury

Vozmožno zadanije aljternativnogo materiala v jego svojstvah.
Vklücenije zameny materiala aljternativnym možno realizovatj globaljno (na vesj urovenj) ili dlä konkretnoj vstrojennoj modeli.


### Mnogoslojnyj effekt (realizovano)

Tekstura generirujetsä každyj kadr iz neskoljkih slojov.
Sloi smešivajutsä drug s drugom zadannym sposobom (Aljfa-test, Aljfa-smešivanije, dobavlenije i t. d.).
Pri etom každyj sloj možno izmenitj - promodulirovatj çvetom, sdvinutj.

V etom variante dlä parametrov sdviga ili moduläçii vozmožno zadanije razlicnyh funkçij.
Vidy funkçj: konstantnaja, linejnaja, sinusoidaljnaja volna, treugoljnaja volna, piloobraznaja volna, kvadratnaja volna.

Necto pohožeje bylo v "Quake III Arena", pravda tam smešivanije vypolnälosj pri rasterizaçii (videokartoj).

V otlicaje ot "Quake III" v "SquareWheel" netu effektov tekstur, takih kak masštabirovanije ili vrascenije.
Takije effekty trebujut vyborki iz tekstury po proizvoljnym koordinatam i vozmožno s filjtraçijej.
Eto bylo by sliškom medlenno.
Bystrymi mogut bytj toljko effekty so sdvigom teksturnyh koordinat na çeluju velicinu.

Ne vpolne ponätno, cto delatj s kartami normalej i šerohovatosti.
Ih transformaçii mogut potrebovatj renormalizaçii.
Vozmožno, stoit ih transformaçii vklücatj toljko jesli eto neobhodimo (javno zadavatj v svojstvah materiala).

Krome sobstvenno osnovnoj tekstury smešivanije po vsem tem že pravilam proishodit i dlä izlucajuscego sloja.

Sloi zadajutsä kak ssylki na materialy.
Eto pozvoläjet bez problem zadatj vse te že parametry tekstur - "diffuse", "normal", "roughness" i t. d., ne dubliruja kod.
Režim smešivanija sloja takže berötsä iz sootvetstvujuscego jemu materiala.


### O proizvoditeljnosti

Effekty, trebujuscije generaçii tekstur každyj kadr, mogut bytj vesjma medlennymi.
Pricina etomu zaklücajetsä v tom, cto effekty primenäjutsä ko vsem teksturam, nezavisimo ot togo, kakije iz nih vidny v tekuscem kadre.
Posemu stoit generirovatj tekstury paralleljno.
Takže ne stoit ispoljzovatj na karte siljno mnogo podobnyh tekstur i/ili sdelatj ih vesjma neboljšogo razrešenija.

Jescö odin sposob snizitj nagruzku - generirovatj tekstury ne každyj kadr.
Dlä etogo suscestvujut speçialjnyje parametry konfiga, kotoryje pozvoläjut sdelatj tak, ctoby každyj kadr obnovlälasj toljko 1/N dolä animirovannyh tekstur.
