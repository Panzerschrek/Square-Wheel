V igre "Unreal" jestj räd dostatocno interesnyh teksturnyh effektov.
Ih interesno bylo by povtoritj.

## Osnovy

Vse nižeopisannyje effekty rabotajut s vosjmibitnyje teksturami.
Pri risovanii vosjmibitnyje znacenija preobrazujutsä v polnoçvetnyje posredstvom unikaljnoj dlä každoj tekstury palitry.

Každaja tekstura obnovläjetsä s fiksirovannoj castotoj.
Eto nužno, ibo skorostj animaçii zadajotsä nejavno castotoj kadrov.


### "FireTexture"

Dlä tekstury vypolnäjetsä algoritm podscöta rasprostranenija ognä - kogda každyj kadr znacenije pikselä vycisläjetsä kak sredneje znacenije cetyröh pikselej pod nim minus nekotoryj parametr zatuhanija.
Na teksturu vlijajut istocniki ognä - t. n. iskry.
Iskr suscestvujet ogromnoje množestvo vidov, u každoj svoja logika.
Nekotoryje iskry mogut poroždatj drugije iskry.

Iskry možno zadatj v redaktore i sohranitj kak svojstvo effekta.

Cerez dannyj effekt realizovany tekstury ognä a takže raznoobraznyh silovyh polej, molnij i t. d.
Eto samyj složnyj effekt - t. k. suscestvujet ogromnoje kolicestvo tipov iskr so svojej logikoj.


### "WaterTexture"

Dlä tekstury vypolnäjetsä algoritm podscöta rasprostranenija voln na poverhnosti vody.
Na vonovoje pole mogut vlijatj istocniki - t. n. kapli, kotoryje mogut dvigatjsä so vremenem.

Kapli možno zadatj v redaktore i sohranitj kak svojstvo effekta.


### "WaveTexture"

Osnovyvajetsä na "WaterTexture".
Na osvove volnovogo polä generirujetsä poverhnostj, osvescönnaja odinocnym istocnikom sveta.


### "WetTexture"

Osnovyvajetsä na "WaterTexture".
Volnovoje pole iskažajet ukazannuju ishodnuju teksturu.
Iskaženije, kažetsä, prosto sdvigajet pikseli vpravo ili vlevo v zavisimosti ot tekuscej amplitudy volny.


### "IceTexture"

Pikseli ishodnoj tekstury sdvigajutsä vpravo ili vlevo na osnove t. n. tekstury stekla.
Tekstura stekla možet so vremenem dvigatjsä.
