Ctoby dobitjsä maksimaljnoj proizvoditeljnosti rasterizatora, nado umenjšitj mnogokratnuju pererisovku pikselej.

Test glubiny ne podhodit dlä etih çelej, t. k. on soderžit popikseljnyje vetvlenija.
Sortirovka ot daljnih poverhnostej k bližnim možet vesti k povyšennoj pererisovke.
Eto ne siljno strašno, no toljko jesli jestj effektivnyj mehanizm ustranenija iz proçessa rasterizaçii nevidimyh poverhnostej.

### Portaljno-sektornyj podhod

Takoj primenälsä v "Duke Nukem 3D", "Thief", "Taras Buljba 3D" i vozmožno gde-to jescö.

Etot podhod pozvoläjet i uporädocitj poverhnosti i otbrositj nevidimyje poverhnosti.
On ne pererisovyvajet ne jedinogo pikselä (dlä osnovnoj geometrii).

Sutj jego primerno sledujuscaja:

Vsä geometroja predstavima kak nabor vypuklyh mnogogrannikov.
Nekotoryje grani - gluhije (steny), nekotoryje že javläjutsä obscimi dlä pary mnogogrannikov (portaly).

Mir risujetsä rekursivno ot mnogogrannika, gde raspoložena kamera.
Snacala risujutsä steny mnogogrannika (porädok ne važnen).
Potom proishodit rekursivnyj zahod v sosednije mnogogranniki cerez portaly i risujutsä uže oni.
Otsecenije proishodit po graniçam portalov.
Dlä nih v pamäti hranitsä dvuhmernyj massiv s nacalom/konçom portala dlä každoj stroki itogovogo izobraženija.

Dinamiceskuju geometriju (modeli) možno narezatj po sektoram i risovatj každuju castj v sootvetstvujuscem sektore.
Poligony v sektore možno risovatj s sortirovkoj ot daljnih k bližnim (cto dobavläjet, inogda, pererisovki).

#### Problema detalej

Sam po sebe portaljno-sektornyh podhod ploh v nekotoryh slucajah, kogda v mire mnogo detalej, osobenno po çentru komnat, ili detalej, vystupajuscih iz sten.
V samom prostom variante takaja geometrija budet narezana na boljšoje kolicestvo sektorov, cto možet zamedlitj rasterizator.

Odno iz vozmožnyh rešenij problemy - isklücenije detaljnoj geometrii iz proçessa postrojenija grafa sektorov.
Risovatj detaljnuju geometriju možno kak i modeli - s narezkoj po sektoram, sortirovkoj i pererisovkoj.

#### Problema otkryrtyh prostranstv

Dlä otobraženija ulicnyh lokaçij, opätj že, možet potrebotatjsä ocenj mnogo sektorov.
Odin iz vozmoužnyh podhodov k ih umenjšeniju - ispojzovanije drugogo podhoda dlä otkrytyh prostranstv, no kotoryj by byl sovmestim s portaljno-sektornym podhodom.

K primeru, jesli kamera nahoditsä vnutri zakrytogo prostranstva (v dome, pescere), to prostranstvo "za oknom" možno risovatj otdeljno, ogranicivaja jego portalom okna.
Jesli že kamera nahoditsä snaruži, to možno narisovatj steny doma vnešnim podhodom, a to, cto vidno cerez okno - portaljnym.

#### Nebo

Nebo v samom prostom variante eto stena s osoboj teksturoj.
V boleje složnom variante eto portal v komnatu s nebesnoj geometrijej (kak v "Unreal" ili "Half-Life 2").

#### Steklo

Možno realizovatj okna so steklom, rešötki i t, d. kak tekstury na portalah.
Pri etom snacala risujetsä geometrija za portalom, a potom tekstura samogo portala.

#### Portaly (v igrovom smysle)

Eto prosto portal, kotoryj logiceski sväzan s sektorom, nahodäscimsä ne za etim portalom, a gde-to v drugom meste.

#### Zerkala

Zerkalo eto portal, smoträscij na samogo sebä i primenäjuscij izmenönnuju matriçu transformaçii dlä geometrii za v nöm.

#### Tuman

T. k. netu bufera glubiny, neljzä delatj tuman postproçessom.
Možno risovatj tuman vmeste s poverhnostämi sektorov.
Rasscityvatj jego intensivnostj poveršinno i interpolirovatj.
Dlä lucšej tocnosti geometriju v sektore s tumanom možno ottseelirovatj.

Poveršinnyj rasscöt tumana primenälsä, k primeru, v dvižke "Quake III".
