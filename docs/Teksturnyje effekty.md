Horošo by imetj v dvižke razlicnyje teksturnyje effekty, ctoby s pomoscuju nih možno bylo realizovatj vsäkije krasivosti.

### Skoljženje tekstur (realizovano)

Samyj prostoj effekt.
K teksturnym koordinatam prosto dobavläjetsä smescenije, linejno zavisimoje ot vremeni.


### "Turb" effekt (realizovano)

Tekstura iskažajetsä sinusoidaljnymi volnami.
Tak delalosj, naprimer, s vodoju v "Quake".
Realizovan effekt cerez zapisj modifiçirovannoj tekstury v otdeljnyj bufer.


### Pokadrovaja animaçija

Tekstura çikliceski menäjetsä vo vremeni s ispoljzovanijem räda izobraženij.
Eto ne trebujet dopolniteljnyh vycislenij - prosto možno každyj kadr menätj indeks v tabliçe tekstur na tekuscij kadr animaçii.
Realizovatj eto možno kak sväzannyj spisok materialov.
Cerez zadanije razlicnogo nacaljnogo materiala možno realizovatj razlicnuju nacaljnuju fazu animaçii u razlicnyh poligonov.


### Smena tekstury

Vozmožno zadanije aljternativnogo materiala v jego svojstvah.
Vklücenije zameny materiala aljternativnym možno realizovatj globaljno (na vesj urovenj) ili dlä konkretnoj vstrojennoj modeli.


### Mnogoslojnyj effekt

Tekstura generirujetsä každyj kadr iz neskoljkih slojov.
Sloi smešivajutsä drug s drugom zadannym sposobom (Aljfa-test, Aljfa-smešivanije, umnoženije i t. d.).
Pri etom každyj sloj možno izmenitj - promodulirovatj çvetom, sdvinutj, primenitj "turb" effekt i t. d.

V etom variante dlä parametrov sdviga ili moduläçii nužno zadanije razlicnyh funkçij.
Vidy funkçj: linejnaja, stupencataja, piloobraznaja, sinusoidaljnaja i t. d.

Necto pohožeje bylo v "Quake III Arena", pravda tam smešivanije vypolnälosj pri rasterizaçii (videokartoj).

V otlicaje ot "Quake III" v "SquareWheel" budet prakticeski nevozmožno primenitj effekty tekstur, takije kak masštabirovanije ili vrascenije.
Takije effekty trebujut vyborki iz tekstury po proizvoljnym koordinatam i vozmožno s filjtraçijej.
Eto bylo by sliškom medlenno.
Bystrymi mogut bytj toljko effekty so sdvigom teksturnyh koordinat na çeluju velicinu.

Ne vpolne ponätno, cto delatj s kartami normalej i šerohovatosti.
Ih transformaçii mogut potrebovatj renormalizaçii.
Vozmožno, stoit ih transformaçii vklücatj toljko jesli eto neobhodimo (javno zadavatj v svojstvah materiala).


### O proizvoditeljnosti

Effekty, trebujuscije generaçii tekstur každyj kadr, mogut bytj vesjma medlennymi.
Pricina etomu zaklücajetsä v tom, cto effekty primenäjutsä ko vsem teksturam, nezavisimo ot togo, kakije iz nih vidny v tekuscem kadre.
Posemu stoit generirovatj tekstury paralleljno.
Takže ne stoit ispoljzovatj na karte siljno mnogo podobnyh tekstur i/ili sdelatj ih vesjma neboljšogo razrešenija.
