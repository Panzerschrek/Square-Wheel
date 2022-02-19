Prakticeski neobhodimo toljko aljfa-smešivanije i additivnoje smešivanije.

## Aljfa-smešivanije

### Usrednenije

Prostejšij vid - smešivanije v ravnyh proporçijah (usrednenije).
Jesli çvetovoj bufer linejen, to eto možno delatj ocenj hitrym sposobom.
Dlä 8-bitnogo bufera:

dst = ((dst & 0xFEFEFEFE) >> 1) + ((src & 0xFEFEFEFE) >> 1);

Ili tak:

dst= (((dst ^ src) & 0xFEFEFEFE) >> 1) + (dst & src);

Dlä bufera v formate R11G11B10 sutj ta že, toljko maski drugije.

### Smešivanije po konstantnomu koeffiçientu

Ne usrednenije, a cto-to drugoje.

V obscem slucaje takoje usrednenije trebujet raspakovki çveta, vypolnenija operaçij nad komponentami i zapakovki obratno.

V castnom slucaje (1/4, 3/4) možno zaispoljzovatj dvuhetapnoje usrednenije.

### Smešivanije po koeffiçientu iz tekstury

Samoje dorogoje smešivanije. Ne podhodit dlä R11G11B10 formata, ibo koeffiçient negde hranitj.

## Zatenenije

Castnyj slucaj aljfa-smešivanija, kogda dobavläjemyj çvet - cörnyj.

### Upolovinivanije jarkosti

Vypolnäjetsä po formule

dst = (dst & 0xFEFEFEFE) >> 1;

Dlä R11G11B10 maska nemnogo drugaja.

### Proizvoljnoje zatemnenije

Eto kogda jarkostj umnožajetsä na kakoje-to cislo meneje jediniçy.
Analogicno, trebujet raspakaovki/zapakovki komponent çveta.

## Additivnoje smešivanije

Pri takom smešivanii proishodit složenije çvetov.
Trebujet raspakovki/zapakovki komponentov.
Trebujet složenija s nasyscenijem.
