# Feature: Progressive Double-Click Selection für einen Texteditor

## Ziel

Erweitere den bestehenden Texteditor um eine intelligente, progressive Textselektion per Doppelklick. Statt bei jedem Doppelklick ausschließlich das Wort unter dem Cursor zu markieren, soll die Auswahl bei wiederholtem Doppelklicken schrittweise auf größere, logisch zusammenhängende Textbereiche erweitert werden.

Das Verhalten soll sich intuitiv anfühlen, wie man es von modernen Code-Editoren und Texteditoren erwartet – jedoch mit einer zusätzlichen, gewichteten Interpretation von Wortgrenzen.

## 1. Grundverhalten

Ein Doppelklick auf eine Textposition startet eine zyklische Selektionssequenz:

1. **Erster Doppelklick:** Selektiere das Wort beziehungsweise Token unter dem Cursor.
2. **Zweiter Doppelklick:** Erweitere die Auswahl um den nächstgelegenen angrenzenden Bereich.
3. **Dritter Doppelklick:** Erweitere die Auswahl erneut, beispielsweise auf den übergeordneten Bezeichner, Ausdruck oder Textabschnitt.
4. **Weitere Doppelklicks:** Erweitere die Auswahl schrittweise, bis ein sinnvoller äußerer Bereich erreicht ist.
5. **Zyklusende:** Beginne wieder mit der ursprünglichen Auswahl des Wortes unter dem Cursor.

Die Auswahl muss bei jedem Schritt nachvollziehbar und möglichst symmetrisch beziehungsweise kontextsensitiv wachsen.

## 2. Wortgrenzen und ihre Gewichtung

Implementiere eine konfigurierbare Klassifikation von Trennzeichen und Wortgrenzen.

Ausgangspunkt ist folgende Priorisierung, die bei Bedarf angepasst werden kann:

| Zeichen / Grenze | Bedeutung | Gewicht |
|---|---|---:|
| Kein Trennzeichen | Zusammenhängender Text | 0 |
| `_` Unterstrich | Trennt Teilbereiche eines Bezeichners | 1 |
| `-` Bindestrich | Trennt zusammengesetzte Begriffe | 2 |
| `.` Punkt | Trennt Bestandteile eines Pfads oder qualifizierten Namens | 3 |
| ` ` Leerzeichen | Trennt Wörter | 4 |
| Tabulator / Zeilenumbruch | Trennt größere Textbereiche | 5 |

Die Gewichtung soll nicht ausschließlich den numerischen Abstand bestimmen. Berücksichtige auch den Textkontext und den jeweiligen Editor-Modus.

Beispiele:

- `user_profile_name`: zuerst `profile`, danach `user_profile` oder `profile_name`, anschließend der gesamte Bezeichner.
- `my-component-name`: zuerst das Wort unter dem Cursor, anschließend das zusammengesetzte Wort und schließlich den gesamten Ausdruck.
- `com.example.service`: zuerst `service`, anschließend `example.service`, danach `com.example.service`.
- `hello world`: zuerst `hello`, danach beispielsweise `hello world` oder der zusammenhängende Textbereich entsprechend der gewählten Expansionsstrategie.
- `src/components/user-profile.ts`: zuerst das aktuelle Token, danach der Dateiname, anschließend Pfadbestandteile und zuletzt der vollständige Pfad.

Wichtig: Die Beispiele dienen als Zielverhalten. Leite daraus eine konsistente Regel ab, statt für einzelne Beispiele Sonderfälle einzubauen.

## 3. Auswahlstrategie

Implementiere einen nachvollziehbaren Algorithmus zur schrittweisen Expansion:

- Ermittle beim ersten Doppelklick die logische Einheit unter der Cursorposition.
- Bestimme anschließend die nächstgelegenen linken und rechten Grenzen.
- Bewerte diese Grenzen anhand ihrer Gewichtung und des Kontexts.
- Erweitere die aktuelle Auswahl um die nächste logische Einheit.
- Bevorzuge zusammenhängende Bereiche und vermeide, dass Zeichen ohne nachvollziehbaren Grund übersprungen werden.
- Berücksichtige Klammern, Anführungszeichen, Satzzeichen, Pfadtrenner und Zeilenumbrüche, sofern sie im jeweiligen Kontext relevant sind.
- Wenn links und rechts gleichwertige Expansionsmöglichkeiten existieren, verwende eine deterministische Regel, beispielsweise die kleinere Erweiterung oder die Seite des ursprünglichen Cursorfokus.
- Wiederhole den Vorgang bis zum maximal sinnvollen Bereich.

Die Auswahl darf sich bei wiederholten Doppelklicks nicht unvorhersehbar verkleinern oder zwischen mehreren Bereichen springen. Nur beim definierten Zyklusneustart wird die ursprüngliche Wortauswahl wiederhergestellt.

## 4. Timing und Erkennung wiederholter Doppelklicks

Unterscheide zwischen:

- einem normalen Doppelklick,
- einem weiteren Doppelklick auf dieselbe Textposition beziehungsweise innerhalb der aktuellen Auswahl,
- einem Doppelklick an einer anderen Textposition,
- einer zwischenzeitlichen manuellen Cursorbewegung oder Änderung der Selektion,
- einer Textänderung zwischen zwei Doppelklicks.

Eine neue Selektionssequenz beginnt, wenn der Benutzer an einer anderen Stelle doppelklickt oder die bestehende Sequenz durch eine andere Interaktion unterbrochen wird.

Verwende geeignete Zeitfenster und Positions-Toleranzen, ohne die native Doppelklick-Erkennung des Betriebssystems unnötig zu umgehen.

Achte besonders darauf, dass normale Mausinteraktionen und die Bedienung mit Tastatur, Touchpad oder Touchscreen nicht beeinträchtigt werden.

## 5. Kontextabhängiges Verhalten

Berücksichtige mindestens diese Textarten:

- **Normaler Fließtext:** Wörter, Wortgruppen und Satzteile.
- **Quellcode:** Bezeichner, zusammengesetzte Bezeichner, qualifizierte Namen und Ausdrücke.
- **Dateipfade und URLs:** Dateinamen, Verzeichnisse, Pfadbestandteile und vollständige Pfade.
- **Konfigurationstexte:** Schlüssel, Werte, Properties und zusammengesetzte Namen.
- **Markdown und ähnliche Formate:** Wörter, Links und gegebenenfalls zusammengehörige Formatierungsbereiche.

Falls der Editor bereits Sprachmodi oder Syntaxinformationen bereitstellt, nutze diese. Implementiere keine aufwendige Spracherkennung, wenn einfache Regeln für den jeweiligen Modus ausreichen.

## 6. Technische Anforderungen

Untersuche zuerst die bestehende Editor-Architektur und identifiziere:

- die verwendete Editor-Komponente,
- die vorhandene Maus- und Doppelklick-Behandlung,
- die API für Textselektionen,
- vorhandene Tastatur- und Cursor-Events,
- Syntaxinformationen und Erweiterungspunkte.

Implementiere die Funktion möglichst als gekapselte Erweiterung, ohne bestehendes Verhalten unnötig zu überschreiben.

Anforderungen:

- Auswahl ausschließlich über die vorhandene Selektions-API des Editors.
- Korrekte Behandlung von Unicode, Umlauten, Emoji und kombinierten Unicode-Zeichen.
- Keine Beschädigung des Textinhalts.
- Keine unnötigen DOM-Manipulationen, sofern der Editor eine eigene Selektions-API bereitstellt.
- Keine Regressionen bei Drag-and-drop, normalem Doppelklick, Cursorbewegung und Tastaturselektion.
- Konfigurierbare Grenzregeln und Gewichtungen, soweit dies zur vorhandenen Architektur passt.
- Gute Performance auch bei großen Dokumenten.

## 7. Tests

Erstelle automatisierte Tests für mindestens folgende Fälle:

1. Einfaches Wort: `hello`
2. Unterstrich: `user_profile_name`
3. Bindestrich: `my-component-name`
4. Punktnotation: `com.example.service`
5. Leerzeichen: `hello world`
6. Gemischter Ausdruck: `user-profile.name_test`
7. Dateipfad: `src/components/user-profile.ts`
8. Satzzeichen und Klammern: `call(user.name)`
9. Deutsche Umlaute: `Königstraße`
10. Unicode und Emoji.
11. Doppelklick auf Anfang, Mitte und Ende eines Tokens.
12. Schnelle und langsamere wiederholte Doppelklicks.
13. Doppelklick an einer anderen Textposition.
14. Unterbrechung durch Cursorbewegung, Tastatureingabe oder Textänderung.
15. Erreichen des äußersten Selektionsbereichs und anschließender Zyklusneustart.

Prüfe nicht nur die finale Auswahl, sondern jeden einzelnen Schritt der Selektionssequenz.

## 8. Umsetzungsvorgehen

1. Analysiere die vorhandene Implementierung.
2. Beschreibe kurz die geeignete Erweiterungsstelle und die geplante Selektionslogik.
3. Implementiere das Feature mit möglichst kleinen, gezielten Änderungen.
4. Ergänze automatisierte Tests.
5. Führe die relevanten Tests und vorhandenen Qualitätsprüfungen aus.
6. Behebe regressionsbedingte Fehler.
7. Dokumentiere die Grenzgewichtung, das Zyklusverhalten und gegebenenfalls notwendige Konfigurationen.

Ändere keine anderen Funktionen ohne Notwendigkeit.

## Abnahmekriterien

Das Feature ist fertig, wenn wiederholte Doppelklicks die Auswahl reproduzierbar, schrittweise und kontextsensitiv erweitern, die definierten Wortgrenzen konsistent behandeln und nach Erreichen des maximalen Bereichs wieder zur ursprünglichen Wortauswahl zurückkehren.

Die Implementierung muss sich nahtlos in den bestehenden Editor integrieren und durch automatisierte Tests abgesichert sein.
