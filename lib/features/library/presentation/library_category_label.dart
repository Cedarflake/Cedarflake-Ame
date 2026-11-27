import "../../../app/presentation/ame_localizations.dart";
import "../domain/library_classification_configuration.dart";

// Curated IDs use resource::identity and managed_category_identity from Rust.
// The topic/purpose roster belongs to resources/classification/curated-topics.json.
String libraryCategoryLabel(
  AppLocalizations strings,
  LibraryClassificationCategory category,
) => switch (category.id) {
  "illustration" => strings.categoryIllustration,
  "comic" => strings.categoryComic,
  "meme" => strings.categoryMeme,
  "screenshot" => strings.categoryScreenshot,
  "photo" => strings.categoryPhoto,
  "other" => strings.categoryOther,
  "29638ff32a3ee6344992d7bf40fec8a0314178e6205610ad57e206070229f48a" =>
    strings.categoryCatsPhoto,
  "2d40dfe0361e1cd234fdee5c101d1b7b72eff877893283be92a64e64a8d4204e" =>
    strings.categoryCatsMeme,
  "14339e04f7993c9fafcb156d3a90dc6d136ceb0415f8f86d7ddbcd9c654b5fc3" =>
    strings.categoryDogsPhoto,
  "f800f31f56283c75826c4f6f0685d3e6adfab250c47bbf0bf0264356fc5e09d2" =>
    strings.categoryDogsMeme,
  "1d0c19cd070a1eaa7b918137577da62cfa2cf1fe3a0f4a07ed6bb694d8e3d95a" =>
    strings.categoryRabbitsPhoto,
  "4467cf8d572dc6728bfc7d5d53ec281d548bc8d6f363955736457d5a334e14bd" =>
    strings.categoryBirdsPhoto,
  "19eab64871ae619ae5fb64a403b96c29b6b6c2cce1a58870f1e8c90ac065dfc7" =>
    strings.categoryWildlifePhoto,
  "53d919300f82e5363019aa329d70e14cf84e786340215e0815b3ab962aa890da" =>
    strings.categoryAnimalCharactersIllustration,
  "4c57507d8e09e1445458476db3f6b75bb0b8a3fee20ce0b77aaa7ce265fcceac" =>
    strings.categoryAnimalCharactersMeme,
  "293afb2004cf177bc89e3a65d05d415b5da3fbfce72f24434f70687eb1017109" =>
    strings.categoryAnimalEaredCharactersIllustration,
  "b449ba283bc287188d677a4ad1945c934c71c78436df8c79e97fd5772c445fda" =>
    strings.categoryAnimalEaredCharactersMeme,
  "78f1c4ed1b96ea69b56854efe9e6787d41290cb46e055d63facfc00292e66d0e" =>
    strings.categoryPortraitsPhoto,
  "add7d83713052549f93c7c4bc4bf46bdf2aa0972370e0815e797fc85a7dfc963" =>
    strings.categoryGroupsPhoto,
  "3c33ba38b8add3b7c89228f309c56904ab1e009a011401809942d03f9d35fd50" =>
    strings.categoryCostumesPhoto,
  "0e94df6a2f23adf7d516b78592972252d21ff7f85391b53d825dd796354b5d13" =>
    strings.categoryPerformancesPhoto,
  "ed988e836d46f7af6f10652100074d6759255f240da27b03ac4918b7ece7c1a6" =>
    strings.categorySportsPhoto,
  "b128372d58c5d9ac6bf71a0f3f78e7d92cde5a325a025a39dc4ba29bbbaaa368" =>
    strings.categoryChildrenPlayingPhoto,
  "0190164b1cc1dc4c6b43add9fdccd518eba4e84d141797deb557ac49f6e2a789" =>
    strings.categoryToysPhoto,
  "0dd7ab3de1670fcc19f7b5485abee4ec318ce966c45aa81db33e58e1d5a92fdb" =>
    strings.categoryToysMeme,
  "b3642c574eec3080310e55f0847cf59db9a5f76a5f98b6be2aa8749e4573a15e" =>
    strings.categoryFoodPhoto,
  "ed55c75e416804b40dd3ef2ed6da74abadbaf1281b4a4f6f6155f150dc1dbd31" =>
    strings.categoryMountainsPhoto,
  "9910105b3298ae609f281fec9eea8371a45726bbd2373037619b2be8734c6b8b" =>
    strings.categoryMountainsIllustration,
  "aeb8eeab8b8a3926a244a892ea22a910f2869c6cf773619d7b1351424c6510ab" =>
    strings.categoryCoastPhoto,
  "a284ea94060ab180bca6779062194ffd6c99ca7e9443e53c1019c042455f8808" =>
    strings.categoryCoastIllustration,
  "d4464490f001fcd7bd0e7975c1bc40baf497066af92e8c8f0c3686e3afba2ddc" =>
    strings.categoryForestsPhoto,
  "3251959df5c25c0399cee701690e8483eaa2e5315ab2d525030a8cf0506bfa9d" =>
    strings.categoryForestsIllustration,
  "951e57580a2a7562cdf95539d06ebb93f09ba39bcffd91950a037a1da1528641" =>
    strings.categoryFlowersPhoto,
  "d4bedae4ebcef5f125fab5a7b0f5f0942b5f04d2d6b66f1cf490024866632fa3" =>
    strings.categoryCityArchitecturePhoto,
  "ee7ebec8c8b6e97b2343ab7da7f841dd9da0ef3db298462d261bbb66d7e67d75" =>
    strings.categoryCityArchitectureIllustration,
  "ab9929a1399994466936a7c50a2622fb7a0cd7687806d292977fe10da87abcc1" =>
    strings.categoryInteriorsPhoto,
  "e3669c074e8f686aa6950ccc396cf4a3c62210542acaa1e2985c07a3eb2c0ffc" =>
    strings.categoryConstructionPhoto,
  "46ffb6cce10b8c08906236703491e649d0729abfe5519ff44a6ba65151fe2a86" =>
    strings.categoryVehiclesPhoto,
  "9b2569567d8128d0a87c0c35a7ceccbf2245354d704f7ad713da6f7a4b936d0f" =>
    strings.categoryProgrammingScreenshot,
  "929390d0abfebc4eec0d802e8f8771f8d960df431958db83bb9839ec23acd664" =>
    strings.categoryMessagingScreenshot,
  "9e6b1ef7d212dc9a853d2eead824dba39849c668f3d15f28fd92c964e93b198a" =>
    strings.categoryGamingScreenshot,
  "a26e3ced5423e7d4501c4c770e889d23a00078b6a5412ed13d2b74bdd970a734" =>
    strings.categoryDocumentReadingScreenshot,
  "6a9d0eb48d6dcf66547b88c6486efbe4549f51735bc4b74c269a7dfaea62f7a9" =>
    strings.categoryDocumentReadingOther,
  "80ad765d069ad8eaa3be48c51be6a2f19b83001d37c742143142be599b2f8742" =>
    strings.categoryApplicationInterfacesScreenshot,
  "6afd8cb93c01ec58b0b303fea20ace5975a267bb267662e4d9538a3660274f0c" =>
    strings.categoryMapsScreenshot,
  "f87e9f6c7b894983d206537f06a43e2f3351fbce6c8e3abd4f3db86fbbb947e4" =>
    strings.categoryMapsOther,
  "5ebc6979d550b494003b480520891109f689b8f006dcc7199581c1b077efc341" =>
    strings.categoryTablesChartsScreenshot,
  "9082c24059714bdfb00d37bafb7c0547a77457dda2529946345b0aee61ab2cb4" =>
    strings.categoryTablesChartsOther,
  "f3b00871d7579509ce9cf896d240dac58c70ef24eee95a51763ec1ba12f1985a" =>
    strings.categoryNotesOther,
  "a9ebfb8ec0e9e17583df5e60f73de665e2d269351c8392a1a87649f19f89e49d" =>
    strings.categoryMathematicsOther,
  "ceeeb9903d569fbe8d07f1639635741a2b324c602959390f87064caa8b5d5116" =>
    strings.categoryMusicOther,
  "a261f6bb9c308b84e2fd80745c8773dc58be95066d7b4a533c4fdcde6373ffa6" =>
    strings.categoryShoppingScreenshot,
  "edab8502199b9d45e31b5f010bb75f9146757194afdc0614d61949c8887d035e" =>
    strings.categoryReceiptsOther,
  "7db160ac7354b63a36a40e1a70f81e86335885a856973c99c2532674ab2710d8" =>
    strings.categoryInterfaceDesignOther,
  "d236f2b3d833576bccc1bb1d8f80fd4ba150f938ca4b4ed1726c04a15efdbc00" =>
    strings.categoryLogosOther,
  "a321d61541bf4b2e3a96e745647f4bfe60537d17f4e108258adfaac7d71b24c8" =>
    strings.categoryFilmScreenshot,
  "05ef61d0e0dbdc305e9e2132b1ad2fa56ec97b11a68f997d8c602b02cc0202dd" =>
    strings.categoryFilmMeme,
  _ => category.label,
};
