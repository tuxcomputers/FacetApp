-- timezone
-- Reference list of IANA time zones (e.g. Australia/Sydney).

CREATE TABLE IF NOT EXISTS timezone (
  timezone_id     INTEGER CONSTRAINT PK_timezone PRIMARY KEY AUTOINCREMENT
  , timezone_name TEXT NOT NULL
  , display_name  TEXT
  , active        INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0,1))
);

CREATE UNIQUE INDEX IF NOT EXISTS UN1_timezone ON timezone(timezone_name);

INSERT INTO timezone (timezone_id, timezone_name, display_name)
SELECT 0, 'Unknown', 'Unknown'
WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 0);

INSERT INTO timezone (timezone_id, timezone_name)
SELECT 1, 'Africa/Abidjan' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 1);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 2, 'Africa/Accra' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 2);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 3, 'Africa/Addis_Ababa' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 3);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 4, 'Africa/Algiers' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 4);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 5, 'Africa/Asmara' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 5);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 6, 'Africa/Bamako' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 6);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 7, 'Africa/Bangui' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 7);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 8, 'Africa/Banjul' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 8);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 9, 'Africa/Bissau' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 9);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 10, 'Africa/Blantyre' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 10);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 11, 'Africa/Brazzaville' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 11);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 12, 'Africa/Bujumbura' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 12);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 13, 'Africa/Cairo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 13);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 14, 'Africa/Casablanca' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 14);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 15, 'Africa/Ceuta' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 15);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 16, 'Africa/Conakry' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 16);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 17, 'Africa/Dakar' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 17);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 18, 'Africa/Dar_es_Salaam' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 18);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 19, 'Africa/Djibouti' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 19);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 20, 'Africa/Douala' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 20);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 21, 'Africa/El_Aaiun' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 21);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 22, 'Africa/Freetown' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 22);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 23, 'Africa/Gaborone' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 23);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 24, 'Africa/Harare' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 24);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 25, 'Africa/Johannesburg' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 25);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 26, 'Africa/Juba' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 26);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 27, 'Africa/Kampala' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 27);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 28, 'Africa/Khartoum' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 28);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 29, 'Africa/Kigali' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 29);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 30, 'Africa/Kinshasa' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 30);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 31, 'Africa/Lagos' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 31);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 32, 'Africa/Libreville' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 32);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 33, 'Africa/Lome' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 33);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 34, 'Africa/Luanda' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 34);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 35, 'Africa/Lubumbashi' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 35);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 36, 'Africa/Lusaka' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 36);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 37, 'Africa/Malabo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 37);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 38, 'Africa/Maputo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 38);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 39, 'Africa/Maseru' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 39);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 40, 'Africa/Mbabane' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 40);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 41, 'Africa/Mogadishu' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 41);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 42, 'Africa/Monrovia' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 42);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 43, 'Africa/Nairobi' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 43);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 44, 'Africa/Ndjamena' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 44);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 45, 'Africa/Niamey' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 45);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 46, 'Africa/Nouakchott' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 46);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 47, 'Africa/Ouagadougou' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 47);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 48, 'Africa/Porto-Novo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 48);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 49, 'Africa/Sao_Tome' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 49);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 50, 'Africa/Tripoli' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 50);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 51, 'Africa/Tunis' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 51);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 52, 'Africa/Windhoek' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 52);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 53, 'America/Adak' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 53);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 54, 'America/Anchorage' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 54);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 55, 'America/Anguilla' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 55);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 56, 'America/Antigua' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 56);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 57, 'America/Araguaina' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 57);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 58, 'America/Argentina/Buenos_Aires' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 58);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 59, 'America/Argentina/Catamarca' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 59);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 60, 'America/Argentina/Cordoba' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 60);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 61, 'America/Argentina/Jujuy' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 61);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 62, 'America/Argentina/La_Rioja' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 62);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 63, 'America/Argentina/Mendoza' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 63);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 64, 'America/Argentina/Rio_Gallegos' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 64);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 65, 'America/Argentina/Salta' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 65);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 66, 'America/Argentina/San_Juan' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 66);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 67, 'America/Argentina/San_Luis' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 67);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 68, 'America/Argentina/Tucuman' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 68);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 69, 'America/Argentina/Ushuaia' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 69);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 70, 'America/Aruba' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 70);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 71, 'America/Asuncion' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 71);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 72, 'America/Atikokan' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 72);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 73, 'America/Bahia' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 73);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 74, 'America/Bahia_Banderas' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 74);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 75, 'America/Barbados' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 75);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 76, 'America/Belem' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 76);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 77, 'America/Belize' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 77);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 78, 'America/Blanc-Sablon' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 78);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 79, 'America/Boa_Vista' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 79);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 80, 'America/Bogota' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 80);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 81, 'America/Boise' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 81);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 82, 'America/Cambridge_Bay' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 82);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 83, 'America/Campo_Grande' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 83);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 84, 'America/Cancun' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 84);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 85, 'America/Caracas' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 85);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 86, 'America/Cayenne' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 86);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 87, 'America/Cayman' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 87);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 88, 'America/Chicago' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 88);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 89, 'America/Chihuahua' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 89);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 90, 'America/Ciudad_Juarez' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 90);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 91, 'America/Costa_Rica' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 91);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 92, 'America/Coyhaique' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 92);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 93, 'America/Creston' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 93);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 94, 'America/Cuiaba' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 94);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 95, 'America/Curacao' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 95);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 96, 'America/Danmarkshavn' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 96);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 97, 'America/Dawson' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 97);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 98, 'America/Dawson_Creek' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 98);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 99, 'America/Denver' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 99);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 100, 'America/Detroit' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 100);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 101, 'America/Dominica' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 101);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 102, 'America/Edmonton' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 102);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 103, 'America/Eirunepe' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 103);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 104, 'America/El_Salvador' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 104);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 105, 'America/Fort_Nelson' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 105);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 106, 'America/Fortaleza' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 106);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 107, 'America/Glace_Bay' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 107);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 108, 'America/Goose_Bay' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 108);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 109, 'America/Grand_Turk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 109);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 110, 'America/Grenada' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 110);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 111, 'America/Guadeloupe' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 111);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 112, 'America/Guatemala' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 112);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 113, 'America/Guayaquil' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 113);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 114, 'America/Guyana' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 114);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 115, 'America/Halifax' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 115);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 116, 'America/Havana' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 116);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 117, 'America/Hermosillo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 117);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 118, 'America/Indiana/Indianapolis' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 118);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 119, 'America/Indiana/Knox' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 119);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 120, 'America/Indiana/Marengo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 120);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 121, 'America/Indiana/Petersburg' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 121);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 122, 'America/Indiana/Tell_City' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 122);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 123, 'America/Indiana/Vevay' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 123);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 124, 'America/Indiana/Vincennes' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 124);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 125, 'America/Indiana/Winamac' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 125);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 126, 'America/Inuvik' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 126);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 127, 'America/Iqaluit' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 127);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 128, 'America/Jamaica' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 128);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 129, 'America/Juneau' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 129);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 130, 'America/Kentucky/Louisville' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 130);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 131, 'America/Kentucky/Monticello' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 131);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 132, 'America/La_Paz' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 132);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 133, 'America/Lima' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 133);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 134, 'America/Los_Angeles' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 134);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 135, 'America/Maceio' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 135);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 136, 'America/Managua' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 136);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 137, 'America/Manaus' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 137);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 138, 'America/Martinique' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 138);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 139, 'America/Matamoros' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 139);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 140, 'America/Mazatlan' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 140);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 141, 'America/Menominee' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 141);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 142, 'America/Merida' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 142);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 143, 'America/Metlakatla' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 143);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 144, 'America/Mexico_City' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 144);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 145, 'America/Miquelon' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 145);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 146, 'America/Moncton' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 146);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 147, 'America/Monterrey' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 147);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 148, 'America/Montevideo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 148);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 149, 'America/Montserrat' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 149);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 150, 'America/Nassau' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 150);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 151, 'America/New_York' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 151);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 152, 'America/Nome' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 152);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 153, 'America/Noronha' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 153);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 154, 'America/North_Dakota/Beulah' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 154);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 155, 'America/North_Dakota/Center' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 155);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 156, 'America/North_Dakota/New_Salem' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 156);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 157, 'America/Nuuk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 157);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 158, 'America/Ojinaga' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 158);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 159, 'America/Panama' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 159);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 160, 'America/Paramaribo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 160);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 161, 'America/Phoenix' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 161);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 162, 'America/Port-au-Prince' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 162);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 163, 'America/Port_of_Spain' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 163);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 164, 'America/Porto_Velho' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 164);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 165, 'America/Puerto_Rico' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 165);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 166, 'America/Punta_Arenas' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 166);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 167, 'America/Rankin_Inlet' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 167);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 168, 'America/Recife' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 168);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 169, 'America/Regina' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 169);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 170, 'America/Resolute' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 170);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 171, 'America/Rio_Branco' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 171);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 172, 'America/Santarem' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 172);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 173, 'America/Santiago' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 173);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 174, 'America/Santo_Domingo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 174);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 175, 'America/Sao_Paulo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 175);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 176, 'America/Scoresbysund' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 176);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 177, 'America/Sitka' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 177);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 178, 'America/St_Johns' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 178);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 179, 'America/St_Kitts' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 179);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 180, 'America/St_Lucia' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 180);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 181, 'America/St_Thomas' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 181);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 182, 'America/St_Vincent' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 182);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 183, 'America/Swift_Current' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 183);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 184, 'America/Tegucigalpa' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 184);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 185, 'America/Thule' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 185);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 186, 'America/Tijuana' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 186);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 187, 'America/Toronto' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 187);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 188, 'America/Tortola' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 188);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 189, 'America/Vancouver' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 189);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 190, 'America/Whitehorse' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 190);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 191, 'America/Winnipeg' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 191);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 192, 'America/Yakutat' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 192);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 193, 'Antarctica/Casey' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 193);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 194, 'Antarctica/Davis' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 194);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 195, 'Antarctica/DumontDUrville' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 195);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 196, 'Antarctica/Macquarie' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 196);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 197, 'Antarctica/Mawson' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 197);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 198, 'Antarctica/McMurdo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 198);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 199, 'Antarctica/Palmer' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 199);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 200, 'Antarctica/Rothera' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 200);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 201, 'Antarctica/Syowa' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 201);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 202, 'Antarctica/Troll' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 202);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 203, 'Antarctica/Vostok' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 203);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 204, 'Asia/Aden' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 204);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 205, 'Asia/Almaty' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 205);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 206, 'Asia/Amman' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 206);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 207, 'Asia/Anadyr' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 207);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 208, 'Asia/Aqtau' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 208);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 209, 'Asia/Aqtobe' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 209);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 210, 'Asia/Ashgabat' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 210);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 211, 'Asia/Atyrau' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 211);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 212, 'Asia/Baghdad' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 212);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 213, 'Asia/Bahrain' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 213);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 214, 'Asia/Baku' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 214);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 215, 'Asia/Bangkok' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 215);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 216, 'Asia/Barnaul' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 216);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 217, 'Asia/Beirut' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 217);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 218, 'Asia/Bishkek' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 218);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 219, 'Asia/Brunei' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 219);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 220, 'Asia/Chita' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 220);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 221, 'Asia/Colombo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 221);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 222, 'Asia/Damascus' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 222);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 223, 'Asia/Dhaka' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 223);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 224, 'Asia/Dili' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 224);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 225, 'Asia/Dubai' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 225);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 226, 'Asia/Dushanbe' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 226);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 227, 'Asia/Famagusta' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 227);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 228, 'Asia/Gaza' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 228);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 229, 'Asia/Hebron' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 229);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 230, 'Asia/Ho_Chi_Minh' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 230);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 231, 'Asia/Hong_Kong' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 231);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 232, 'Asia/Hovd' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 232);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 233, 'Asia/Irkutsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 233);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 234, 'Asia/Jakarta' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 234);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 235, 'Asia/Jayapura' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 235);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 236, 'Asia/Jerusalem' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 236);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 237, 'Asia/Kabul' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 237);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 238, 'Asia/Kamchatka' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 238);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 239, 'Asia/Karachi' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 239);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 240, 'Asia/Kathmandu' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 240);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 241, 'Asia/Khandyga' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 241);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 242, 'Asia/Kolkata' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 242);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 243, 'Asia/Krasnoyarsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 243);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 244, 'Asia/Kuala_Lumpur' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 244);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 245, 'Asia/Kuching' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 245);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 246, 'Asia/Kuwait' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 246);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 247, 'Asia/Macau' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 247);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 248, 'Asia/Magadan' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 248);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 249, 'Asia/Makassar' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 249);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 250, 'Asia/Manila' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 250);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 251, 'Asia/Muscat' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 251);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 252, 'Asia/Nicosia' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 252);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 253, 'Asia/Novokuznetsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 253);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 254, 'Asia/Novosibirsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 254);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 255, 'Asia/Omsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 255);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 256, 'Asia/Oral' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 256);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 257, 'Asia/Phnom_Penh' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 257);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 258, 'Asia/Pontianak' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 258);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 259, 'Asia/Pyongyang' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 259);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 260, 'Asia/Qatar' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 260);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 261, 'Asia/Qostanay' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 261);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 262, 'Asia/Qyzylorda' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 262);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 263, 'Asia/Riyadh' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 263);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 264, 'Asia/Sakhalin' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 264);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 265, 'Asia/Samarkand' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 265);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 266, 'Asia/Seoul' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 266);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 267, 'Asia/Shanghai' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 267);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 268, 'Asia/Singapore' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 268);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 269, 'Asia/Srednekolymsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 269);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 270, 'Asia/Taipei' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 270);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 271, 'Asia/Tashkent' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 271);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 272, 'Asia/Tbilisi' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 272);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 273, 'Asia/Tehran' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 273);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 274, 'Asia/Thimphu' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 274);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 275, 'Asia/Tokyo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 275);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 276, 'Asia/Tomsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 276);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 277, 'Asia/Ulaanbaatar' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 277);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 278, 'Asia/Urumqi' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 278);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 279, 'Asia/Ust-Nera' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 279);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 280, 'Asia/Vientiane' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 280);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 281, 'Asia/Vladivostok' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 281);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 282, 'Asia/Yakutsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 282);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 283, 'Asia/Yangon' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 283);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 284, 'Asia/Yekaterinburg' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 284);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 285, 'Asia/Yerevan' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 285);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 286, 'Atlantic/Azores' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 286);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 287, 'Atlantic/Bermuda' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 287);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 288, 'Atlantic/Canary' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 288);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 289, 'Atlantic/Cape_Verde' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 289);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 290, 'Atlantic/Faroe' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 290);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 291, 'Atlantic/Madeira' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 291);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 292, 'Atlantic/Reykjavik' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 292);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 293, 'Atlantic/South_Georgia' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 293);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 294, 'Atlantic/St_Helena' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 294);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 295, 'Atlantic/Stanley' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 295);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 296, 'Australia/Adelaide' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 296);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 297, 'Australia/Brisbane' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 297);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 298, 'Australia/Broken_Hill' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 298);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 299, 'Australia/Darwin' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 299);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 300, 'Australia/Eucla' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 300);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 301, 'Australia/Hobart' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 301);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 302, 'Australia/Lindeman' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 302);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 303, 'Australia/Lord_Howe' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 303);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 304, 'Australia/Melbourne' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 304);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 305, 'Australia/Perth' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 305);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 306, 'Australia/Sydney' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 306);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 307, 'CET' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 307);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 308, 'CST6CDT' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 308);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 309, 'EET' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 309);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 310, 'EST' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 310);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 311, 'EST5EDT' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 311);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 312, 'Etc/GMT' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 312);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 313, 'Etc/GMT+1' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 313);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 314, 'Etc/GMT+10' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 314);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 315, 'Etc/GMT+11' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 315);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 316, 'Etc/GMT+12' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 316);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 317, 'Etc/GMT+2' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 317);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 318, 'Etc/GMT+3' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 318);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 319, 'Etc/GMT+4' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 319);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 320, 'Etc/GMT+5' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 320);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 321, 'Etc/GMT+6' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 321);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 322, 'Etc/GMT+7' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 322);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 323, 'Etc/GMT+8' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 323);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 324, 'Etc/GMT+9' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 324);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 325, 'Etc/GMT-1' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 325);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 326, 'Etc/GMT-10' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 326);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 327, 'Etc/GMT-11' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 327);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 328, 'Etc/GMT-12' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 328);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 329, 'Etc/GMT-13' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 329);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 330, 'Etc/GMT-14' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 330);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 331, 'Etc/GMT-2' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 331);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 332, 'Etc/GMT-3' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 332);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 333, 'Etc/GMT-4' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 333);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 334, 'Etc/GMT-5' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 334);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 335, 'Etc/GMT-6' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 335);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 336, 'Etc/GMT-7' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 336);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 337, 'Etc/GMT-8' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 337);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 338, 'Etc/GMT-9' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 338);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 339, 'Etc/UTC' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 339);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 340, 'Europe/Amsterdam' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 340);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 341, 'Europe/Andorra' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 341);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 342, 'Europe/Astrakhan' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 342);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 343, 'Europe/Athens' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 343);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 344, 'Europe/Belgrade' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 344);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 345, 'Europe/Berlin' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 345);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 346, 'Europe/Brussels' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 346);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 347, 'Europe/Bucharest' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 347);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 348, 'Europe/Budapest' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 348);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 349, 'Europe/Chisinau' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 349);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 350, 'Europe/Copenhagen' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 350);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 351, 'Europe/Dublin' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 351);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 352, 'Europe/Gibraltar' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 352);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 353, 'Europe/Guernsey' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 353);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 354, 'Europe/Helsinki' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 354);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 355, 'Europe/Isle_of_Man' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 355);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 356, 'Europe/Istanbul' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 356);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 357, 'Europe/Jersey' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 357);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 358, 'Europe/Kaliningrad' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 358);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 359, 'Europe/Kirov' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 359);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 360, 'Europe/Kyiv' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 360);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 361, 'Europe/Lisbon' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 361);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 362, 'Europe/Ljubljana' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 362);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 363, 'Europe/London' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 363);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 364, 'Europe/Luxembourg' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 364);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 365, 'Europe/Madrid' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 365);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 366, 'Europe/Malta' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 366);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 367, 'Europe/Minsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 367);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 368, 'Europe/Monaco' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 368);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 369, 'Europe/Moscow' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 369);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 370, 'Europe/Oslo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 370);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 371, 'Europe/Paris' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 371);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 372, 'Europe/Prague' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 372);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 373, 'Europe/Riga' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 373);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 374, 'Europe/Rome' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 374);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 375, 'Europe/Samara' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 375);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 376, 'Europe/Sarajevo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 376);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 377, 'Europe/Saratov' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 377);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 378, 'Europe/Simferopol' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 378);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 379, 'Europe/Skopje' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 379);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 380, 'Europe/Sofia' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 380);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 381, 'Europe/Stockholm' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 381);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 382, 'Europe/Tallinn' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 382);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 383, 'Europe/Tirane' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 383);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 384, 'Europe/Ulyanovsk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 384);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 385, 'Europe/Vaduz' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 385);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 386, 'Europe/Vienna' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 386);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 387, 'Europe/Vilnius' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 387);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 388, 'Europe/Volgograd' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 388);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 389, 'Europe/Warsaw' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 389);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 390, 'Europe/Zagreb' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 390);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 391, 'Europe/Zurich' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 391);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 392, 'Factory' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 392);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 393, 'HST' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 393);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 394, 'Indian/Antananarivo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 394);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 395, 'Indian/Chagos' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 395);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 396, 'Indian/Christmas' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 396);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 397, 'Indian/Cocos' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 397);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 398, 'Indian/Comoro' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 398);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 399, 'Indian/Kerguelen' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 399);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 400, 'Indian/Mahe' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 400);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 401, 'Indian/Maldives' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 401);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 402, 'Indian/Mauritius' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 402);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 403, 'Indian/Mayotte' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 403);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 404, 'Indian/Reunion' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 404);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 405, 'MET' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 405);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 406, 'MST' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 406);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 407, 'MST7MDT' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 407);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 408, 'PST8PDT' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 408);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 409, 'Pacific/Apia' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 409);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 410, 'Pacific/Auckland' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 410);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 411, 'Pacific/Bougainville' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 411);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 412, 'Pacific/Chatham' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 412);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 413, 'Pacific/Chuuk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 413);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 414, 'Pacific/Easter' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 414);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 415, 'Pacific/Efate' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 415);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 416, 'Pacific/Fakaofo' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 416);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 417, 'Pacific/Fiji' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 417);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 418, 'Pacific/Funafuti' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 418);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 419, 'Pacific/Galapagos' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 419);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 420, 'Pacific/Gambier' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 420);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 421, 'Pacific/Guadalcanal' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 421);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 422, 'Pacific/Guam' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 422);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 423, 'Pacific/Honolulu' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 423);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 424, 'Pacific/Kanton' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 424);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 425, 'Pacific/Kiritimati' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 425);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 426, 'Pacific/Kosrae' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 426);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 427, 'Pacific/Kwajalein' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 427);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 428, 'Pacific/Majuro' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 428);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 429, 'Pacific/Marquesas' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 429);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 430, 'Pacific/Midway' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 430);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 431, 'Pacific/Nauru' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 431);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 432, 'Pacific/Niue' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 432);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 433, 'Pacific/Norfolk' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 433);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 434, 'Pacific/Noumea' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 434);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 435, 'Pacific/Pago_Pago' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 435);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 436, 'Pacific/Palau' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 436);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 437, 'Pacific/Pitcairn' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 437);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 438, 'Pacific/Pohnpei' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 438);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 439, 'Pacific/Port_Moresby' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 439);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 440, 'Pacific/Rarotonga' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 440);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 441, 'Pacific/Saipan' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 441);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 442, 'Pacific/Tahiti' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 442);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 443, 'Pacific/Tarawa' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 443);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 444, 'Pacific/Tongatapu' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 444);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 445, 'Pacific/Wake' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 445);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 446, 'Pacific/Wallis' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 446);
INSERT INTO timezone (timezone_id, timezone_name)
SELECT 447, 'WET' WHERE NOT EXISTS (SELECT 1 FROM timezone WHERE timezone_id = 447);
