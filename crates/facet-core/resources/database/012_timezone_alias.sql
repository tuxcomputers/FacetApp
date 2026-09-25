-- timezone_alias
-- Every legacy IANA name, against the timezone row that replaced it.

CREATE TABLE IF NOT EXISTS timezone_alias (
  timezone_alias_id     INTEGER CONSTRAINT PK_timezone_alias PRIMARY KEY
  , timezone_alias_name TEXT NOT NULL
  , timezone_id         INTEGER NOT NULL DEFAULT 0 REFERENCES timezone(timezone_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS UN1_timezone_alias ON timezone_alias(timezone_alias_name);

INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 1, 'Africa/Asmera', 5 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 1);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 2, 'Africa/Timbuktu', 6 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 2);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 3, 'America/Argentina/ComodRivadavia', 59 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 3);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 4, 'America/Atka', 53 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 4);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 5, 'America/Buenos_Aires', 58 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 5);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 6, 'America/Catamarca', 59 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 6);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 7, 'America/Coral_Harbour', 72 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 7);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 8, 'America/Cordoba', 60 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 8);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 9, 'America/Ensenada', 186 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 9);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 10, 'America/Fort_Wayne', 118 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 10);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 11, 'America/Godthab', 157 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 11);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 12, 'America/Indianapolis', 118 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 12);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 13, 'America/Jujuy', 61 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 13);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 14, 'America/Knox_IN', 119 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 14);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 15, 'America/Kralendijk', 95 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 15);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 16, 'America/Louisville', 130 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 16);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 17, 'America/Lower_Princes', 95 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 17);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 18, 'America/Marigot', 163 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 18);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 19, 'America/Mendoza', 63 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 19);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 20, 'America/Montreal', 187 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 20);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 21, 'America/Nipigon', 187 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 21);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 22, 'America/Pangnirtung', 127 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 22);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 23, 'America/Porto_Acre', 171 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 23);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 24, 'America/Rainy_River', 191 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 24);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 25, 'America/Rosario', 60 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 25);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 26, 'America/Santa_Isabel', 186 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 26);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 27, 'America/Shiprock', 99 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 27);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 28, 'America/St_Barthelemy', 163 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 28);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 29, 'America/Thunder_Bay', 187 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 29);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 30, 'America/Virgin', 181 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 30);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 31, 'America/Yellowknife', 102 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 31);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 32, 'Antarctica/South_Pole', 198 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 32);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 33, 'Arctic/Longyearbyen', 370 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 33);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 34, 'Asia/Ashkhabad', 210 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 34);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 35, 'Asia/Calcutta', 242 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 35);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 36, 'Asia/Choibalsan', 277 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 36);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 37, 'Asia/Chongqing', 267 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 37);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 38, 'Asia/Chungking', 267 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 38);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 39, 'Asia/Dacca', 223 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 39);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 40, 'Asia/Harbin', 267 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 40);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 41, 'Asia/Istanbul', 356 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 41);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 42, 'Asia/Kashgar', 278 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 42);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 43, 'Asia/Katmandu', 240 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 43);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 44, 'Asia/Macao', 247 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 44);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 45, 'Asia/Rangoon', 283 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 45);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 46, 'Asia/Saigon', 230 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 46);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 47, 'Asia/Tel_Aviv', 236 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 47);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 48, 'Asia/Thimbu', 274 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 48);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 49, 'Asia/Ujung_Pandang', 249 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 49);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 50, 'Asia/Ulan_Bator', 277 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 50);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 51, 'Atlantic/Faeroe', 290 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 51);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 52, 'Atlantic/Jan_Mayen', 370 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 52);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 53, 'Australia/ACT', 306 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 53);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 54, 'Australia/Canberra', 306 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 54);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 55, 'Australia/Currie', 301 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 55);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 56, 'Australia/LHI', 303 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 56);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 57, 'Australia/NSW', 306 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 57);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 58, 'Australia/North', 299 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 58);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 59, 'Australia/Queensland', 297 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 59);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 60, 'Australia/South', 296 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 60);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 61, 'Australia/Tasmania', 301 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 61);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 62, 'Australia/Victoria', 304 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 62);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 63, 'Australia/West', 305 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 63);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 64, 'Australia/Yancowinna', 298 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 64);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 65, 'Brazil/Acre', 171 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 65);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 66, 'Brazil/DeNoronha', 153 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 66);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 67, 'Brazil/East', 175 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 67);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 68, 'Brazil/West', 137 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 68);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 69, 'Canada/Atlantic', 115 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 69);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 70, 'Canada/Central', 191 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 70);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 71, 'Canada/Eastern', 187 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 71);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 72, 'Canada/Mountain', 102 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 72);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 73, 'Canada/Newfoundland', 178 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 73);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 74, 'Canada/Pacific', 189 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 74);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 75, 'Canada/Saskatchewan', 169 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 75);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 76, 'Canada/Yukon', 190 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 76);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 77, 'Chile/Continental', 173 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 77);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 78, 'Chile/EasterIsland', 414 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 78);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 79, 'Cuba', 116 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 79);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 80, 'Egypt', 13 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 80);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 81, 'Eire', 351 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 81);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 82, 'Etc/GMT+0', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 82);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 83, 'Etc/GMT-0', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 83);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 84, 'Etc/GMT0', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 84);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 85, 'Etc/Greenwich', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 85);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 86, 'Etc/UCT', 339 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 86);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 87, 'Etc/Universal', 339 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 87);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 88, 'Etc/Zulu', 339 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 88);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 89, 'Europe/Belfast', 363 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 89);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 90, 'Europe/Bratislava', 372 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 90);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 91, 'Europe/Busingen', 391 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 91);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 92, 'Europe/Kiev', 360 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 92);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 93, 'Europe/Mariehamn', 354 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 93);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 94, 'Europe/Nicosia', 252 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 94);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 95, 'Europe/Podgorica', 344 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 95);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 96, 'Europe/San_Marino', 374 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 96);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 97, 'Europe/Tiraspol', 349 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 97);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 98, 'Europe/Uzhgorod', 360 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 98);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 99, 'Europe/Vatican', 374 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 99);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 100, 'Europe/Zaporozhye', 360 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 100);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 101, 'GB', 363 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 101);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 102, 'GB-Eire', 363 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 102);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 103, 'GMT', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 103);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 104, 'GMT+0', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 104);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 105, 'GMT-0', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 105);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 106, 'GMT0', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 106);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 107, 'Greenwich', 312 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 107);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 108, 'Hongkong', 231 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 108);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 109, 'Iceland', 292 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 109);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 110, 'Iran', 273 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 110);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 111, 'Israel', 236 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 111);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 112, 'Jamaica', 128 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 112);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 113, 'Japan', 275 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 113);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 114, 'Kwajalein', 427 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 114);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 115, 'Libya', 50 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 115);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 116, 'Mexico/BajaNorte', 186 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 116);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 117, 'Mexico/BajaSur', 140 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 117);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 118, 'Mexico/General', 144 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 118);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 119, 'NZ', 410 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 119);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 120, 'NZ-CHAT', 412 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 120);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 121, 'Navajo', 99 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 121);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 122, 'PRC', 267 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 122);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 123, 'Pacific/Enderbury', 424 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 123);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 124, 'Pacific/Johnston', 423 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 124);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 125, 'Pacific/Ponape', 438 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 125);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 126, 'Pacific/Samoa', 435 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 126);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 127, 'Pacific/Truk', 413 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 127);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 128, 'Pacific/Yap', 413 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 128);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 129, 'Poland', 389 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 129);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 130, 'Portugal', 361 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 130);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 131, 'ROC', 270 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 131);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 132, 'ROK', 266 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 132);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 133, 'Singapore', 268 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 133);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 134, 'Turkey', 356 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 134);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 135, 'UCT', 339 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 135);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 136, 'US/Alaska', 54 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 136);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 137, 'US/Aleutian', 53 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 137);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 138, 'US/Arizona', 161 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 138);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 139, 'US/Central', 88 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 139);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 140, 'US/East-Indiana', 118 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 140);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 141, 'US/Eastern', 151 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 141);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 142, 'US/Hawaii', 423 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 142);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 143, 'US/Indiana-Starke', 119 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 143);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 144, 'US/Michigan', 100 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 144);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 145, 'US/Mountain', 99 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 145);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 146, 'US/Pacific', 134 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 146);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 147, 'US/Samoa', 435 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 147);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 148, 'UTC', 339 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 148);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 149, 'Universal', 339 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 149);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 150, 'W-SU', 369 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 150);
INSERT INTO timezone_alias (timezone_alias_id, timezone_alias_name, timezone_id)
SELECT 151, 'Zulu', 339 WHERE NOT EXISTS (SELECT 1 FROM timezone_alias WHERE timezone_alias_id = 151);
