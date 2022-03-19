Po [materialu](http://nothings.org/gamedev/thief_rendering.html) ([perevod](https://habr.com/ru/post/321986/)).

### O risovanii v "Thief"

V "Thief" isploljzovalasj tehnologija sovmescenija portalov/sektorov i dereva dvoicnogo razbijenija prostranstva.

Graf sektorov obhodilsä vglubj i dlä každogo sektora sohranälsä ogranicivajuscij vosjmiugoljnik (v prostranstve ekrana), cerez kotoryj on viden.
Daleje poligony mira risovalisj ot daljnih k bližnim putöm obhoda dereva dvoicnogo razbijenija prostranstva.
Poligony pri etom obrezalisj vosjmiugoljnikami sektorov, v kotoryh oni nahodätsä.
Modeli otcasti uporädocivalisj - risovalisj poperemenno s sektorami, a otcasti narezalisj.


### O podhode kak takovom

Dannyj podhod privodit inogda k pererisovkam pikselej.
No pererisovok ne siljno mnogo, t. k. vo-pervyh nevidimaja geometrija otsekajetsä, a vo-vtoryh, poligony obrazajutsä po vosjmiugoljnikam.

Bufer glubiny ne nužen, cto ekonomit resursy na jego zapolnenije.
Detaljnaja geometrija možet risovatjsä tak že, kak i modeli vnutri sektora - s sortirovkoj.
Vidimyje poverhnosti opredeläjutsä ne absolütno tocno, a približönno, cto možet privoditj k lišnemu postrojeniju tekstur poverhnostej.

Sam podhod ne trebujet rucnoj razbivki na sektora i oboznacenija portalov.
Sektora strojatä iz listjev dereva dvoicnogo zarbijenija prostranstva.
S odnoj storony eto dostoinstvo - uproscäjetsä redaktirovanije urovnej.
S drugoj storony nalicije dvoicnogo razbijenija prostranstva možet privoditj k izlišnej narezke poligonov.
Zadaca postrojenija grafa sektorov/portalov iz dereva dvoicnogo razbijenija prostranstva ne prosta.

Jescö jestj problmena otsecenija nevidimyh poverhnostej pri postrojenii dereva iz brašej (v formate kart "Quake").
Jestj predpoloženije, cto nevidimyje poverhnosti za graniçami urovä budut obrazovyvatj sektora, nedostižimyje iz osnovnyh sektorov urovnä.
V takom slucaje takije sektora možno budet prosto otbrositj obhodom grafa - vybrositj izolirovannyje podgrafy.

Risovanije poverhnostej po derevu dvoicnogo razbijenija prostranstva imejet preimuscestvo nad podhodom s risovanijem poverhnostej pri obhode dereva sektorov.
V etom variante poverhnostj, vidimaja cerez neskoljko raznyh putej, risujetsä odin raz.
Odin raz vypolnäjetsä transformaçija veršin.
Pri etom kolicestvo zalityh pikselej budet primerno tem že ili boljšim, v hudših slucajah - siljno boljšim.