use hex_literal::hex;
use primitive_types::U256;
use sp_core::H256;

//** DEV **
// Vector initial configuration
pub const BROADCASTER_DOMAIN: u32 = 2;
pub const BROADCASTER: H256 = H256(hex!(
	"1369A4C9391cF90D393b40fAeAD521b0F7019dc5000000000000000000000000" // Sepolia broadcaster address
));
pub const SLOTS_PER_PERIOD: u64 = 8192;
pub const FINALITY_THRESHOLD: u16 = 342;
pub const PERIOD: u64 = 566;
// Attention!
// If Posidon hash starts with "0" it (the zero character) will be ignore.
// This means that that hash needs to be manually added to the chain spec.
// 
// "May fortune favor the foolish." - Captain James T. Kirk (Star Trek)
pub fn get_poseidon_hash_for_period() -> U256 {
	// PERIOD hash
	U256::from(hex!(
		"03bb653a74fff851a48ad577e9bdafd6ab8b73726b7f3056f0e3c94b456de502"
	))
}
pub const GENESIS_VALIDATOR_ROOT: H256 = H256(hex!(
	"d8ea171f3c94aea21ebc42a1ed61052acf3f9209c00e4efbaaddac09ed9b8078"
));
pub const GENESIS_TIME: u64 = 1655733600;
pub const SECONDS_PER_SLOT: u64 = 12;
pub const SOURCE_CHAIN_ID: u64 = 11155111;
pub const STEP_FUNCTION_ID: H256 = H256(hex!(
	"55b63fe87aef4a2d5e6a141455c12964f2b5611a45a30104fc78cbda308c0ee3"
));
pub const ROTATE_FUNCTION_ID: H256 = H256(hex!(
	"a511bd86a30fa6db581480ac7591d4271c845411ac4e1ad93797d09a57b60522"
));
pub const STEP_VK: &str = r#"{"vk_json":{
 "protocol": "groth16",
 "curve": "bn128",
 "nPublic": 2,
 "vk_alpha_1": [
  "20491192805390485299153009773594534940189261866228447918068658471970481763042",
  "9383485363053290200918347156157836566562967994039712273449902621266178545958",
  "1"
 ],
 "vk_beta_2": [
  [
   "6375614351688725206403948262868962793625744043794305715222011528459656738731",
   "4252822878758300859123897981450591353533073413197771768651442665752259397132"
  ],
  [
   "10505242626370262277552901082094356697409835680220590971873171140371331206856",
   "21847035105528745403288232691147584728191162732299865338377159692350059136679"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_gamma_2": [
  [
   "10857046999023057135944570762232829481370756359578518086990519993285655852781",
   "11559732032986387107991004021392285783925812861821192530917403151452391805634"
  ],
  [
   "8495653923123431417604973247489272438418190587263600148770280649306958101930",
   "4082367875863433681332203403145435568316851327593401208105741076214120093531"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_delta_2": [
  [
   "1511229334563333960614051397721255204540116529871718126069540030087143912668",
   "1186967640332823652532824638974572497576318474207196193880591000013362107322"
  ],
  [
   "11054322621635544717695193760165695811106514148528489964124609414760036252189",
   "18322724109150063049119801954602683424454596634898906513132333232963732523822"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_alphabeta_12": [
  [
   [
    "2029413683389138792403550203267699914886160938906632433982220835551125967885",
    "21072700047562757817161031222997517981543347628379360635925549008442030252106"
   ],
   [
    "5940354580057074848093997050200682056184807770593307860589430076672439820312",
    "12156638873931618554171829126792193045421052652279363021382169897324752428276"
   ],
   [
    "7898200236362823042373859371574133993780991612861777490112507062703164551277",
    "7074218545237549455313236346927434013100842096812539264420499035217050630853"
   ]
  ],
  [
   [
    "7077479683546002997211712695946002074877511277312570035766170199895071832130",
    "10093483419865920389913245021038182291233451549023025229112148274109565435465"
   ],
   [
    "4595479056700221319381530156280926371456704509942304414423590385166031118820",
    "19831328484489333784475432780421641293929726139240675179672856274388269393268"
   ],
   [
    "11934129596455521040620786944827826205713621633706285934057045369193958244500",
    "8037395052364110730298837004334506829870972346962140206007064471173334027475"
   ]
  ]
 ],
 "IC": [
  [
   "15722483674559934742116422659315901974716129126941140805990334086131081718976",
   "3902134898294519190003285840720221830219610064345538063642832921891334916446",
   "1"
  ],
  [
   "15938607518268350705291559624750354588934585294828316762040730112905695732765",
   "9404654170873340825490492928605726794760787854641143281657170513374309992357",
   "1"
  ],
  [
   "8312495127808904415217428347989069507040216564083959678081296227931560230026",
   "1580445039973631634117301246726646019573382183752517345585457344955714318714",
   "1"
  ]
 ]
}}"#;
pub const ROTATE_VK: &str = r#"{"vk_json":{
 "protocol": "groth16",
 "curve": "bn128",
 "nPublic": 2,
 "vk_alpha_1": [
  "20491192805390485299153009773594534940189261866228447918068658471970481763042",
  "9383485363053290200918347156157836566562967994039712273449902621266178545958",
  "1"
 ],
 "vk_beta_2": [
  [
   "6375614351688725206403948262868962793625744043794305715222011528459656738731",
   "4252822878758300859123897981450591353533073413197771768651442665752259397132"
  ],
  [
   "10505242626370262277552901082094356697409835680220590971873171140371331206856",
   "21847035105528745403288232691147584728191162732299865338377159692350059136679"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_gamma_2": [
  [
   "10857046999023057135944570762232829481370756359578518086990519993285655852781",
   "11559732032986387107991004021392285783925812861821192530917403151452391805634"
  ],
  [
   "8495653923123431417604973247489272438418190587263600148770280649306958101930",
   "4082367875863433681332203403145435568316851327593401208105741076214120093531"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_delta_2": [
  [
   "2864156988502350018268114524769442611229738724281856064310359811414088775164",
   "19784911050814990253005325251017779746002278450060367709911093357779852409724"
  ],
  [
   "2320747355788118605608963241136772405889379999161258135797985959373766905799",
   "7118041328407665643077665093375077236507031390654037220453830314560753892708"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_alphabeta_12": [
  [
   [
    "2029413683389138792403550203267699914886160938906632433982220835551125967885",
    "21072700047562757817161031222997517981543347628379360635925549008442030252106"
   ],
   [
    "5940354580057074848093997050200682056184807770593307860589430076672439820312",
    "12156638873931618554171829126792193045421052652279363021382169897324752428276"
   ],
   [
    "7898200236362823042373859371574133993780991612861777490112507062703164551277",
    "7074218545237549455313236346927434013100842096812539264420499035217050630853"
   ]
  ],
  [
   [
    "7077479683546002997211712695946002074877511277312570035766170199895071832130",
    "10093483419865920389913245021038182291233451549023025229112148274109565435465"
   ],
   [
    "4595479056700221319381530156280926371456704509942304414423590385166031118820",
    "19831328484489333784475432780421641293929726139240675179672856274388269393268"
   ],
   [
    "11934129596455521040620786944827826205713621633706285934057045369193958244500",
    "8037395052364110730298837004334506829870972346962140206007064471173334027475"
   ]
  ]
 ],
 "IC": [
  [
   "15615341388138779177592192310982411536626378440854127969627902314302018589756",
   "15825561397777957655855081872509949298182852212017977148985160662370122761845",
   "1"
  ],
  [
   "21866659777455953012076240694890418723891531368136637553921599064988704009798",
   "18794682133425820197214508210971026410261369883290190279860606526851568182754",
   "1"
  ],
  [
   "17134706853007662603932468543386586959990776778768283640697616786730646170163",
   "20580957029031123131958004810864543174606183854578157485523871304119815226629",
   "1"
  ]
 ]
}}"#;

//** PROD **
// Vector init config
pub const PROD_BROADCASTER_DOMAIN: u32 = 2;
pub const PROD_BROADCASTER: H256 = H256(hex!(
	"Aa8c1bFC413e00884A7ac991851686D27b387997000000000000000000000000" // Sepolia address
));
pub const PROD_SLOTS_PER_PERIOD: u64 = 8192;
pub const PROD_FINALITY_THRESHOLD: u16 = 342;
pub const PROD_PERIOD: u64 = 526;
pub fn prod_get_poseidon_hash_for_period() -> U256 {
	// PERIOD hash
	U256::from(hex!(
		"20d4234c2adca715b9b7c7d3eb3f8d9230fc97fa036e14dd9f050cd2010e0492"
	))
}
pub const PROD_GENESIS_VALIDATOR_ROOT: H256 = H256(hex!(
	"4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95"
));
pub const PROD_GENESIS_TIME: u64 = 1606824023;
pub const PROD_SECONDS_PER_SLOT: u64 = 12;
pub const PROD_SOURCE_CHAIN_ID: u64 = 1;
pub const PROD_STEP_FUNCTION_ID: H256 = H256(hex!(
	"22405eefd595d2057393ef9c27a3694839a58b5121cac7e41ed9123a56930c8b"
));
pub const PROD_ROTATE_FUNCTION_ID: H256 = H256(hex!(
	"c7e13ba912a18fc047292e7698bb2af9c8a2eb901886a785b2cb4b9fd2394573"
));
pub const PROD_STEP_VK: &str = r#"{"vk_json":{
    "protocol": "groth16",
    "curve": "bn128",
    "nPublic": 2,
    "vk_alpha_1": [
     "20491192805390485299153009773594534940189261866228447918068658471970481763042",
     "9383485363053290200918347156157836566562967994039712273449902621266178545958",
     "1"
    ],
    "vk_beta_2": [
     [
      "6375614351688725206403948262868962793625744043794305715222011528459656738731",
      "4252822878758300859123897981450591353533073413197771768651442665752259397132"
     ],
     [
      "10505242626370262277552901082094356697409835680220590971873171140371331206856",
      "21847035105528745403288232691147584728191162732299865338377159692350059136679"
     ],
     [
      "1",
      "0"
     ]
    ],
    "vk_gamma_2": [
     [
      "10857046999023057135944570762232829481370756359578518086990519993285655852781",
      "11559732032986387107991004021392285783925812861821192530917403151452391805634"
     ],
     [
      "8495653923123431417604973247489272438418190587263600148770280649306958101930",
      "4082367875863433681332203403145435568316851327593401208105741076214120093531"
     ],
     [
      "1",
      "0"
     ]
    ],
    "vk_delta_2": [
     [
      "677302577815076814357170457144294271294364985082280272249076505900964830740",
      "5628948730667472013190771331033856457010306836153142947462627646651446565415"
     ],
     [
      "5877290568297658003612857476419103064356778304319760331670835003648166891449",
      "10874997846396459971354014654692242947705540424071616448481145872912634110727"
     ],
     [
      "1",
      "0"
     ]
    ],
    "vk_alphabeta_12": [
     [
      [
       "2029413683389138792403550203267699914886160938906632433982220835551125967885",
       "21072700047562757817161031222997517981543347628379360635925549008442030252106"
      ],
      [
       "5940354580057074848093997050200682056184807770593307860589430076672439820312",
       "12156638873931618554171829126792193045421052652279363021382169897324752428276"
      ],
      [
       "7898200236362823042373859371574133993780991612861777490112507062703164551277",
       "7074218545237549455313236346927434013100842096812539264420499035217050630853"
      ]
     ],
     [
      [
       "7077479683546002997211712695946002074877511277312570035766170199895071832130",
       "10093483419865920389913245021038182291233451549023025229112148274109565435465"
      ],
      [
       "4595479056700221319381530156280926371456704509942304414423590385166031118820",
       "19831328484489333784475432780421641293929726139240675179672856274388269393268"
      ],
      [
       "11934129596455521040620786944827826205713621633706285934057045369193958244500",
       "8037395052364110730298837004334506829870972346962140206007064471173334027475"
      ]
     ]
    ],
    "IC": [
     [
      "202333273032481017331373350816007583026713320195536354260471885571526195724",
      "8246242704115088390751476790768744984402990892657920674334938931948100192840",
      "1"
     ],
     [
      "12901454334783146822957332552289769626984444933652541503990843020723194328882",
      "12436078488518552293095332739673622487901350475115357313978341690183990059269",
      "1"
     ],
     [
      "12828056956769114977702246128118682473179646035440405756936949778100648490262",
      "7351319165217643779735289066901404053730163225836026220896225559268517203790",
      "1"
     ]
    ]
   }}"#;
pub const PROD_ROTATE_VK: &str = r#"{"vk_json":{
 "protocol": "groth16",
 "curve": "bn128",
 "nPublic": 2,
 "vk_alpha_1": [
  "20491192805390485299153009773594534940189261866228447918068658471970481763042",
  "9383485363053290200918347156157836566562967994039712273449902621266178545958",
  "1"
 ],
 "vk_beta_2": [
  [
   "6375614351688725206403948262868962793625744043794305715222011528459656738731",
   "4252822878758300859123897981450591353533073413197771768651442665752259397132"
  ],
  [
   "10505242626370262277552901082094356697409835680220590971873171140371331206856",
   "21847035105528745403288232691147584728191162732299865338377159692350059136679"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_gamma_2": [
  [
   "10857046999023057135944570762232829481370756359578518086990519993285655852781",
   "11559732032986387107991004021392285783925812861821192530917403151452391805634"
  ],
  [
   "8495653923123431417604973247489272438418190587263600148770280649306958101930",
   "4082367875863433681332203403145435568316851327593401208105741076214120093531"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_delta_2": [
  [
   "2864156988502350018268114524769442611229738724281856064310359811414088775164",
   "19784911050814990253005325251017779746002278450060367709911093357779852409724"
  ],
  [
   "2320747355788118605608963241136772405889379999161258135797985959373766905799",
   "7118041328407665643077665093375077236507031390654037220453830314560753892708"
  ],
  [
   "1",
   "0"
  ]
 ],
 "vk_alphabeta_12": [
  [
   [
    "2029413683389138792403550203267699914886160938906632433982220835551125967885",
    "21072700047562757817161031222997517981543347628379360635925549008442030252106"
   ],
   [
    "5940354580057074848093997050200682056184807770593307860589430076672439820312",
    "12156638873931618554171829126792193045421052652279363021382169897324752428276"
   ],
   [
    "7898200236362823042373859371574133993780991612861777490112507062703164551277",
    "7074218545237549455313236346927434013100842096812539264420499035217050630853"
   ]
  ],
  [
   [
    "7077479683546002997211712695946002074877511277312570035766170199895071832130",
    "10093483419865920389913245021038182291233451549023025229112148274109565435465"
   ],
   [
    "4595479056700221319381530156280926371456704509942304414423590385166031118820",
    "19831328484489333784475432780421641293929726139240675179672856274388269393268"
   ],
   [
    "11934129596455521040620786944827826205713621633706285934057045369193958244500",
    "8037395052364110730298837004334506829870972346962140206007064471173334027475"
   ]
  ]
 ],
 "IC": [
  [
   "15615341388138779177592192310982411536626378440854127969627902314302018589756",
   "15825561397777957655855081872509949298182852212017977148985160662370122761845",
   "1"
  ],
  [
   "21866659777455953012076240694890418723891531368136637553921599064988704009798",
   "18794682133425820197214508210971026410261369883290190279860606526851568182754",
   "1"
  ],
  [
   "17134706853007662603932468543386586959990776778768283640697616786730646170163",
   "20580957029031123131958004810864543174606183854578157485523871304119815226629",
   "1"
  ]
 ]
}}"#;
