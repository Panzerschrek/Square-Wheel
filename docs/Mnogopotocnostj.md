Bylo by glupo ne ispoljzovatj mnogopotocnostj dlä programmnogogo rasterizatora.
V ideale jejo ispoljzovanije možet ubystritj risovanije kadrov v kolicestvo raz, ravnoje kolicestvu jader proçessora.

## O etapah postrojenija kadra

Kadr stroitsä ne srazu, a poetapno. Suscestvujut sledujuscije etapy, no eto ne polnyj spisok:

* Pervicnyj obhod grafa sektorov, postrojenije spiska vidimyh sektorov i poverhnostej.
  Etot etap neobhodim, t. k. prosto pri rekursivnom obhode sektorov poverhnosti stroitj neljzä, ibo možet byjti dublirovanije.
  Takže etot podhod neobhodim dlä nahoždenija istocnikov sveta, vlijajuscih na sçenu.
* Risovanije kart tenej dlä vidimyh istocnikov.
* Podgotovka poverhnostej.
* Vtoricnyj obhod grafa sektorov s risovanijem poverhnostej.
* Postobrabotka ekrannogo bufera.

Pocti na každom etape rabotu možno vypolnätj paralleljno.
Poverhnosti možno stroitj paralleljno, ibo oni vse nezavisimy.
Analogicno paralleljno možno stroitj karty tenej dlä istocnikov sveta.
Neposredstvenno risovanije poverhnostej možno rasparallelitj, razdeliv ekrannyj bufer na casti, ctoby každyj potok risoval toljko v svoju castj ekrana.
Postobrabotku možno vypolnätj analogicno, po castäm.

Pod somnenijem stoit zovmožnostj paralleljnogo postrojenija spiska vidimyh sektorov.
No, vozmožno, i eto možno sdelatj, jesli zaispoljzovatj kakuju-nibudj prostuju strukturu dannyh s ispoljzovanijem atomarnyh peremennyh.
Vozmožno, takže, cto etot etap budet vesjma bystrym i jego rasparallelivatj vovse ne budet nužno.

## O modeli mnogopotocnosti

Vo mnogih igrah ("Doom 3 BFG", "Doom 4") primenäjetsä modelj mnogopotocnosti na osnove zadac.
Eto kogda suscestvujet nekij nabor potokov i svoj planirovscik, kotoryj raspredeläjet zadaci po potokam.
Kod, kotoryj cto-to delajet, razdeläjetsä na zadaci.

Dumaju, stoit poprobovatj takoj podhod, vozmožno, vzäv gotovuju biblioteku dlä etogo.

https://github.com/taskflow/taskflow
https://github.com/GameTechDev/GTS-GamesTaskScheduler