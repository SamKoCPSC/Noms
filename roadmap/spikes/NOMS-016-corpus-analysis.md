# NOMS-016 Corpus Analysis

This report contains aggregate measurements, structural classifications, and sample IDs only. It does not reproduce recipe body content.

## Summary

- Providers: 15
- Samples: 1500
- Plain-text bytes: 13,344,920
- HTML bytes: 543,873,822
- Total bytes: 557,218,742
- Integrity verified: yes

## Key findings

- Integrity checks passed for all 1500 paired plain-text and HTML samples.
- Boolean structural signatures produced 153 clusters; 15 of 15 providers have more than one observed page shape.
- Ingredient and instruction headings were detected in 1498 and 1498 samples, respectively.
- Sanitization-relevant remote resources, scripts, and iframes were detected in 1500, 1215, and 1027 samples.

## Provider payloads

| Provider | Samples | Clusters | Text p50 | Text p95 | HTML p50 | HTML p95 | HTML max |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| AllRecipes | 100 | 13 | 4,391 | 17,235 | 272,515 | 456,835 | 515,541 |
| BBCGoodFood | 100 | 5 | 4,148 | 5,924 | 681,941 | 728,545 | 752,794 |
| BonAppetit | 100 | 5 | 5,794 | 9,726 | 892,515 | 1,031,196 | 1,139,395 |
| BudgetBytes | 100 | 9 | 17,968 | 23,231 | 460,598 | 504,443 | 528,551 |
| Epicurious | 100 | 6 | 5,612 | 8,317 | 609,820 | 668,477 | 778,954 |
| Food.com | 100 | 5 | 3,434 | 7,156 | 191,835 | 323,090 | 562,156 |
| Food52 | 100 | 3 | 4,284 | 7,109 | 101,723 | 150,965 | 491,523 |
| FoodNetwork | 100 | 11 | 3,998 | 5,068 | 252,073 | 265,431 | 396,298 |
| KingArthurBaking | 100 | 3 | 6,560 | 9,319 | 257,299 | 277,796 | 364,487 |
| NYTCooking | 100 | 6 | 9,756 | 10,779 | 563,553 | 611,725 | 640,513 |
| RecipeTinEats | 100 | 38 | 23,164 | 34,899 | 307,457 | 419,825 | 437,430 |
| SallysBakingAddiction | 100 | 21 | 19,683 | 27,402 | 238,355 | 311,076 | 520,150 |
| SeriousEats | 100 | 13 | 8,862 | 17,843 | 206,741 | 252,005 | 382,880 |
| SimplyRecipes | 100 | 12 | 6,727 | 10,272 | 191,666 | 216,045 | 334,062 |
| Tasty | 100 | 3 | 4,450 | 6,067 | 123,137 | 136,631 | 144,771 |

### Complete payload distributions

Percentiles use the nearest-rank method; means are rounded to whole bytes.

| Provider | Payload | Min | p50 | p90 | p95 | Max | Mean |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| AllRecipes | plainText | 1,598 | 4,391 | 8,534 | 17,235 | 21,119 | 5,922 |
| AllRecipes | html | 221,069 | 272,515 | 300,549 | 456,835 | 515,541 | 288,486 |
| AllRecipes | total | 226,380 | 277,805 | 307,138 | 474,070 | 532,848 | 294,408 |
| BBCGoodFood | plainText | 3,132 | 4,148 | 5,179 | 5,924 | 10,429 | 4,348 |
| BBCGoodFood | html | 619,711 | 681,941 | 711,765 | 728,545 | 752,794 | 677,786 |
| BBCGoodFood | total | 624,243 | 685,399 | 716,294 | 733,706 | 757,276 | 682,135 |
| BonAppetit | plainText | 4,116 | 5,794 | 8,371 | 9,726 | 11,459 | 6,307 |
| BonAppetit | html | 858,130 | 892,515 | 982,763 | 1,031,196 | 1,139,395 | 915,181 |
| BonAppetit | total | 862,246 | 897,706 | 989,786 | 1,038,498 | 1,149,121 | 921,488 |
| BudgetBytes | plainText | 8,679 | 17,968 | 22,638 | 23,231 | 26,807 | 18,064 |
| BudgetBytes | html | 385,620 | 460,598 | 495,851 | 504,443 | 528,551 | 458,385 |
| BudgetBytes | total | 400,070 | 478,964 | 516,459 | 527,081 | 555,358 | 476,449 |
| Epicurious | plainText | 3,480 | 5,612 | 7,736 | 8,317 | 11,365 | 5,861 |
| Epicurious | html | 574,272 | 609,820 | 645,284 | 668,477 | 778,954 | 617,519 |
| Epicurious | total | 577,756 | 615,597 | 653,210 | 674,860 | 785,824 | 623,380 |
| Food.com | plainText | 1,893 | 3,434 | 5,729 | 7,156 | 12,145 | 3,971 |
| Food.com | html | 106,649 | 191,835 | 297,014 | 323,090 | 562,156 | 214,191 |
| Food.com | total | 110,392 | 197,972 | 306,446 | 330,597 | 572,499 | 218,162 |
| Food52 | plainText | 1,976 | 4,284 | 6,191 | 7,109 | 9,341 | 4,334 |
| Food52 | html | 88,219 | 101,723 | 112,857 | 150,965 | 491,523 | 108,416 |
| Food52 | total | 90,809 | 105,830 | 119,361 | 155,684 | 495,281 | 112,750 |
| FoodNetwork | plainText | 2,102 | 3,998 | 4,780 | 5,068 | 8,210 | 3,957 |
| FoodNetwork | html | 224,427 | 252,073 | 261,882 | 265,431 | 396,298 | 254,969 |
| FoodNetwork | total | 226,529 | 255,897 | 266,105 | 271,922 | 400,380 | 258,926 |
| KingArthurBaking | plainText | 4,234 | 6,560 | 8,642 | 9,319 | 11,940 | 6,814 |
| KingArthurBaking | html | 228,944 | 257,299 | 270,151 | 277,796 | 364,487 | 259,096 |
| KingArthurBaking | total | 233,178 | 264,051 | 278,383 | 288,761 | 375,101 | 265,910 |
| NYTCooking | plainText | 7,949 | 9,756 | 10,588 | 10,779 | 11,677 | 9,685 |
| NYTCooking | html | 472,243 | 563,553 | 600,450 | 611,725 | 640,513 | 561,581 |
| NYTCooking | total | 480,822 | 573,367 | 610,341 | 621,898 | 650,859 | 571,265 |
| RecipeTinEats | plainText | 6,255 | 23,164 | 30,357 | 34,899 | 43,989 | 22,385 |
| RecipeTinEats | html | 133,382 | 307,457 | 413,269 | 419,825 | 437,430 | 306,877 |
| RecipeTinEats | total | 141,932 | 316,688 | 442,183 | 453,704 | 473,401 | 329,263 |
| SallysBakingAddiction | plainText | 11,586 | 19,683 | 26,399 | 27,402 | 32,053 | 19,960 |
| SallysBakingAddiction | html | 158,628 | 238,355 | 301,073 | 311,076 | 520,150 | 244,943 |
| SallysBakingAddiction | total | 172,672 | 255,593 | 323,504 | 334,543 | 541,827 | 264,902 |
| SeriousEats | plainText | 4,880 | 8,862 | 13,550 | 17,843 | 36,776 | 10,149 |
| SeriousEats | html | 174,106 | 206,741 | 233,504 | 252,005 | 382,880 | 213,780 |
| SeriousEats | total | 179,865 | 215,595 | 244,401 | 271,987 | 419,311 | 223,929 |
| SimplyRecipes | plainText | 4,615 | 6,727 | 8,588 | 10,272 | 17,005 | 7,216 |
| SimplyRecipes | html | 173,443 | 191,666 | 206,614 | 216,045 | 334,062 | 196,211 |
| SimplyRecipes | total | 178,598 | 198,541 | 214,542 | 227,112 | 350,190 | 203,427 |
| Tasty | plainText | 2,761 | 4,450 | 5,621 | 6,067 | 8,334 | 4,477 |
| Tasty | html | 100,630 | 123,137 | 129,356 | 136,631 | 144,771 | 121,317 |
| Tasty | total | 103,391 | 127,613 | 134,149 | 141,913 | 151,772 | 125,793 |

## Structural feature counts

| Provider | Clipboard fragment markers | Schema.org Recipe markup | Recipe/title heading | Ingredient region | Instruction region | Grouped ingredients | Ingredient list markup | Instruction list markup | Duplicate ingredient regions | Duplicate instruction regions | Form controls | Unit controls | Yield/servings controls | Notes region | Equipment region | Images | Remote resource references | Scripts | Style markup | Inline event handlers | Iframes | Links | JSON-LD |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| AllRecipes | 0 | 0 | 100 | 100 | 100 | 6 | 100 | 100 | 29 | 0 | 100 | 0 | 99 | 60 | 1 | 100 | 100 | 10 | 100 | 26 | 100 | 100 | 0 |
| BBCGoodFood | 0 | 100 | 100 | 100 | 100 | 29 | 0 | 100 | 0 | 100 | 100 | 99 | 0 | 100 | 1 | 100 | 100 | 100 | 100 | 7 | 100 | 100 | 100 |
| BonAppetit | 0 | 0 | 100 | 100 | 100 | 20 | 0 | 100 | 1 | 96 | 100 | 0 | 0 | 100 | 2 | 100 | 100 | 100 | 100 | 0 | 100 | 100 | 0 |
| BudgetBytes | 0 | 0 | 100 | 100 | 100 | 45 | 74 | 99 | 18 | 100 | 100 | 0 | 100 | 96 | 0 | 100 | 100 | 100 | 100 | 100 | 100 | 100 | 0 |
| Epicurious | 0 | 0 | 100 | 100 | 100 | 22 | 0 | 100 | 2 | 93 | 100 | 0 | 0 | 100 | 2 | 100 | 100 | 100 | 100 | 0 | 100 | 100 | 0 |
| Food.com | 0 | 100 | 100 | 100 | 100 | 9 | 100 | 100 | 1 | 0 | 100 | 100 | 100 | 0 | 0 | 100 | 100 | 100 | 100 | 0 | 10 | 100 | 100 |
| Food52 | 0 | 0 | 100 | 100 | 100 | 100 | 66 | 83 | 100 | 0 | 100 | 0 | 0 | 100 | 100 | 100 | 100 | 100 | 100 | 0 | 100 | 100 | 0 |
| FoodNetwork | 0 | 3 | 100 | 100 | 100 | 23 | 100 | 80 | 100 | 2 | 100 | 0 | 21 | 100 | 100 | 100 | 100 | 100 | 100 | 0 | 3 | 100 | 3 |
| KingArthurBaking | 0 | 0 | 100 | 100 | 100 | 4 | 100 | 100 | 4 | 15 | 100 | 0 | 0 | 100 | 0 | 100 | 100 | 100 | 100 | 0 | 100 | 100 | 0 |
| NYTCooking | 0 | 100 | 100 | 100 | 100 | 98 | 98 | 100 | 1 | 0 | 100 | 0 | 0 | 0 | 0 | 100 | 100 | 100 | 100 | 0 | 3 | 100 | 0 |
| RecipeTinEats | 0 | 0 | 100 | 98 | 98 | 87 | 79 | 35 | 81 | 10 | 100 | 71 | 38 | 97 | 0 | 100 | 100 | 100 | 100 | 100 | 86 | 100 | 0 |
| SallysBakingAddiction | 0 | 0 | 100 | 100 | 100 | 90 | 95 | 100 | 86 | 10 | 100 | 0 | 4 | 100 | 65 | 100 | 100 | 100 | 100 | 100 | 23 | 100 | 0 |
| SeriousEats | 0 | 0 | 100 | 100 | 100 | 2 | 100 | 100 | 4 | 8 | 100 | 1 | 0 | 44 | 90 | 100 | 100 | 2 | 100 | 99 | 100 | 100 | 0 |
| SimplyRecipes | 0 | 0 | 100 | 100 | 100 | 21 | 100 | 100 | 48 | 4 | 100 | 0 | 0 | 61 | 0 | 100 | 100 | 3 | 100 | 100 | 100 | 100 | 0 |
| Tasty | 0 | 0 | 100 | 100 | 100 | 87 | 100 | 100 | 100 | 0 | 100 | 0 | 0 | 100 | 0 | 100 | 100 | 100 | 100 | 0 | 2 | 100 | 0 |

## Fixture recommendations

Selections start with each provider's dominant cluster, then maximize structural distance across remaining clusters before filling from dominant shapes. Each provider contributes at most five sample IDs.

| Provider | Recommended sample IDs | Fixture priorities |
| --- | --- | --- |
| AllRecipes | AllRecipes/Baby Spinach Omelet Recipe<br>AllRecipes/Easy Meatloaf<br>AllRecipes/Turkey in a Bag Recipe<br>AllRecipes/Old Fashioned Pea Salad Recipe<br>AllRecipes/Buttered Noodles Recipe | Exercise duplicate ingredient-region disambiguation (29/100 samples).<br>Exercise yield controls without importing form chrome.<br>Preserve grouped ingredient boundaries (6/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| BBCGoodFood | BBCGoodFood/Air fryer quesadillas recipe Good Food<br>BBCGoodFood/Cheesy black bean quesadillas recipe Good Food<br>BBCGoodFood/Air fryer Bramley apple crumble recipe Good Food<br>BBCGoodFood/Brilliant Banana Loaf<br>BBCGoodFood/Choc-orange energy boosters recipe Good Food | Exercise duplicate instruction-region disambiguation (100/100 samples).<br>Exercise unit controls without importing form chrome.<br>Preserve grouped ingredient boundaries (29/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization.<br>Compare semantic Recipe markup with the visible structural fallback. |
| BonAppetit | BonAppetit/Anything-Goes Donabe<br>BonAppetit/Banana Pudding Paris-Brest Recipe<br>BonAppetit/2-Ingredient Chocolate Mousse Recipe<br>BonAppetit/BA's Best Lasagna<br>BonAppetit/Artichoke Beurre Recipe | Exercise duplicate ingredient-region disambiguation (1/100 samples).<br>Exercise duplicate instruction-region disambiguation (96/100 samples).<br>Preserve grouped ingredient boundaries (20/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| BudgetBytes | BudgetBytes/BBQ Chicken Foil Packs<br>BudgetBytes/Easy Minestrone Soup<br>BudgetBytes/Breakfast Pizza<br>BudgetBytes/Balsamic Roasted Mushrooms with Herby Kale Mashed Potatoes<br>BudgetBytes/Bibimbap (Korean Mixed Rice Bowl) | Exercise duplicate ingredient-region disambiguation (18/100 samples).<br>Exercise duplicate instruction-region disambiguation (100/100 samples).<br>Exercise yield controls without importing form chrome.<br>Preserve grouped ingredient boundaries (45/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| Epicurious | Epicurious/Baked Feta Pasta With Green Sauce Recipe<br>Epicurious/Korean-Style Stuffed Peppers Recipe<br>Epicurious/2-Ingredient Chocolate Mousse Recipe<br>Epicurious/A Better Beer Can Chicken Recipe<br>Epicurious/Artichoke Beurre Recipe | Exercise duplicate ingredient-region disambiguation (2/100 samples).<br>Exercise duplicate instruction-region disambiguation (93/100 samples).<br>Preserve grouped ingredient boundaries (22/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| Food.com | Food.com/! Arriba ! Baked Winter Squash Mexican-Style Recipe<br>Food.com/Flaky Pie Crust<br>Food.com/Apple Crisp<br>Food.com/#1 Best Vegan--Style Granola in the World! Recipe<br>Food.com/0 Point Ice Cream- Only 1 Ingredient Recipe | Exercise duplicate ingredient-region disambiguation (1/100 samples).<br>Exercise unit and yield controls without importing form chrome (100/100 samples expose unit signals; 100/100 expose yield signals).<br>Preserve grouped ingredient boundaries (9/100 samples).<br>Remove active and remote page resources during sanitization.<br>Compare semantic Recipe markup with the visible structural fallback. |
| Food52 | Food52/Air-Fryer Chile-Cheese Toast<br>Food52/Avocado Deviled Eggs with Smoked Salmon and Fried Capers<br>Food52/Air-Fryer Gochujang Chicken Wings<br>Food52/Air-Fryer Korean-Style Beef Tacos<br>Food52/Balsamic Vinaigrette-Marinated Skirt Steak With Tomato Salad | Exercise duplicate ingredient-region disambiguation (100/100 samples).<br>Preserve grouped ingredient boundaries (100/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| FoodNetwork | FoodNetwork/10 Minute Apple Sauce Recipe Alton Brown<br>FoodNetwork/Chicken Parmesan<br>FoodNetwork/16-Minute Greek Feast To Go Recipe Ree Drummond<br>FoodNetwork/1-2-3 Lasagna Recipe<br>FoodNetwork/1 Dish Salmon + Veggies Recipe Molly Yeh | Exercise duplicate ingredient-region disambiguation (100/100 samples).<br>Exercise duplicate instruction-region disambiguation (2/100 samples).<br>Exercise yield controls without importing form chrome.<br>Preserve grouped ingredient boundaries (23/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization.<br>Compare semantic Recipe markup with the visible structural fallback. |
| KingArthurBaking | KingArthurBaking/100% Whole Wheat Hamburger Buns Recipe<br>KingArthurBaking/Brown Sugar Strawberry Frosted Cake Recipe<br>KingArthurBaking/Apple Cinnamon Sourdough Bread Recipe<br>KingArthurBaking/American Flag Pie Recipe<br>KingArthurBaking/BLT Pizza Recipe | Exercise duplicate ingredient-region disambiguation (4/100 samples).<br>Exercise duplicate instruction-region disambiguation (15/100 samples).<br>Preserve grouped ingredient boundaries (4/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| NYTCooking | NYTCooking/Baked Ziti<br>NYTCooking/No-Knead Bread<br>NYTCooking/Shakshuka with Feta<br>NYTCooking/Cranberry Curd Tart Recipe<br>NYTCooking/Five-Ingredient Creamy Miso Pasta Recipe | Exercise duplicate ingredient-region disambiguation (1/100 samples).<br>Preserve grouped ingredient boundaries (98/100 samples).<br>Remove active and remote page resources during sanitization.<br>Compare semantic Recipe markup with the visible structural fallback. |
| RecipeTinEats | RecipeTinEats/A Very Big Pumpkin Layer Cake - with Toffee Pecans and Cream Cheese Frosting<br>RecipeTinEats/Christmas Cooking Hotline 2025!<br>RecipeTinEats/Butter Chicken<br>RecipeTinEats/Herbes de Provence<br>RecipeTinEats/Baharat spice blend (Lebanese 7 Spice) | Exercise missing or ambiguous recipe regions (2 ingredient; 2 instruction).<br>Exercise duplicate ingredient-region disambiguation (81/100 samples).<br>Exercise duplicate instruction-region disambiguation (10/100 samples).<br>Exercise unit and yield controls without importing form chrome (71/100 samples expose unit signals; 38/100 expose yield signals).<br>Preserve grouped ingredient boundaries (87/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| SallysBakingAddiction | SallysBakingAddiction/9x13-Inch Pan Brownie Recipe - Sally's Baking<br>SallysBakingAddiction/Make-Ahead Gingerbread Cinnamon Rolls - Sally's Baking<br>SallysBakingAddiction/Homemade Blueberry Bagels (Real Flavor) - Sally's Baking<br>SallysBakingAddiction/Homemade Whipped Cream - Sally's Baking<br>SallysBakingAddiction/How to Freeze Bananas for Baking - Sally's Baking | Exercise duplicate ingredient-region disambiguation (86/100 samples).<br>Exercise duplicate instruction-region disambiguation (10/100 samples).<br>Exercise yield controls without importing form chrome.<br>Preserve grouped ingredient boundaries (90/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| SeriousEats | SeriousEats/Asparagus Quiche Recipe<br>SeriousEats/Scottish-Style Oatmeal<br>SeriousEats/Foolproof Pan Pizza<br>SeriousEats/Date Salad With Spicy Tahini-Yogurt Dressing<br>SeriousEats/Homemade Cream-Filled Doughnuts Recipe | Exercise duplicate ingredient-region disambiguation (4/100 samples).<br>Exercise duplicate instruction-region disambiguation (8/100 samples).<br>Exercise unit controls without importing form chrome.<br>Preserve grouped ingredient boundaries (2/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| SimplyRecipes | SimplyRecipes/Amish Macaroni Salad Recipe (Make Ahead)<br>SimplyRecipes/2-Ingredient Banana Pancakes Recipe<br>SimplyRecipes/Chicken Pot Pie<br>SimplyRecipes/Coconut Creamed Corn Recipe (3 Ingredients, 20 Minutes)<br>SimplyRecipes/Banana Bread | Exercise duplicate ingredient-region disambiguation (48/100 samples).<br>Exercise duplicate instruction-region disambiguation (4/100 samples).<br>Preserve grouped ingredient boundaries (21/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |
| Tasty | Tasty/2-Ingredient Frozen Mango And Pineapple Ice Cream<br>Tasty/Easy Brazilian Lemonade Cocktail (Limonada Suíça)<br>Tasty/One-Pot Garlic Parmesan Pasta<br>Tasty/3 Fruity Summer Refreshers (Blended Fruit Drinks)<br>Tasty/50-Clove Garlic And Cheese Layer Potatoes | Exercise duplicate ingredient-region disambiguation (100/100 samples).<br>Preserve grouped ingredient boundaries (87/100 samples).<br>Separate notes and equipment from ingredients and instructions.<br>Remove active and remote page resources during sanitization. |

## Structural clusters

### AllRecipes

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| allrecipes-01 | 28 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+yieldControls+images+remoteResources+styles+iframes+links | AllRecipes/Baby Spinach Omelet Recipe<br>AllRecipes/Baked Flounder with Panko and Parmesan Recipe<br>AllRecipes/Best Salmon Bake Recipe |
| allrecipes-02 | 26 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+yieldControls+notes+images+remoteResources+styles+iframes+links | AllRecipes/Air Fryer Baked Potatoes Recipe<br>AllRecipes/Baked Fresh Rainbow Trout Recipe<br>AllRecipes/Baked Haddock Recipe |
| allrecipes-03 | 15 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+images+remoteResources+styles+iframes+links | AllRecipes/Air Fryer Chicken Thighs Recipe<br>AllRecipes/Baked Chicken Schnitzel Recipe<br>AllRecipes/Best Ever Crab Cakes Recipe |
| allrecipes-04 | 9 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+yieldControls+images+remoteResources+styles+eventHandlers+iframes+links | AllRecipes/Classic Cuban-Style Picadillo Recipe<br>AllRecipes/Crispy Panko Chicken Breasts Recipe<br>AllRecipes/Fiesta Tater Tot Casserole Recipe |
| allrecipes-05 | 6 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | AllRecipes/Banana Banana Bread<br>AllRecipes/Best Chocolate Chip Cookies<br>AllRecipes/Chicken Pot Pie |
| allrecipes-06 | 6 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+yieldControls+notes+images+remoteResources+styles+eventHandlers+iframes+links | AllRecipes/Old Fashioned Pea Salad Recipe<br>AllRecipes/Pan-Fried Pork Chops Recipe<br>AllRecipes/Pasta Roni Recipe |
| allrecipes-07 | 3 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | AllRecipes/Easy Meatloaf<br>AllRecipes/Homemade Mac and Cheese<br>AllRecipes/World's Best Lasagna |
| allrecipes-08 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+images+remoteResources+styles+iframes+links | AllRecipes/Buttered Noodles Recipe<br>AllRecipes/Creamed Chipped Beef on Toast Recipe |
| allrecipes-09 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+images+remoteResources+styles+iframes+links | AllRecipes/Easy 5-Ingredient Salmon Recipe |
| allrecipes-10 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+equipment+images+remoteResources+styles+iframes+links | AllRecipes/Turkey in a Bag Recipe |
| allrecipes-11 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+images+remoteResources+styles+eventHandlers+iframes+links | AllRecipes/Simple Deep Fried Turkey Recipe |
| allrecipes-12 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+images+remoteResources+styles+iframes+links | AllRecipes/Sheet Pan Brunch Totchos Recipe |
| allrecipes-13 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+yieldControls+images+remoteResources+scripts+styles+eventHandlers+iframes+links | AllRecipes/Slow Cooker Beef Stew |

### BBCGoodFood

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| bbcgoodfood-01 | 63 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+instructionLists+duplicateInstructionRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+iframes+links+jsonLd | BBCGoodFood/Air fryer quesadillas recipe Good Food<br>BBCGoodFood/Air-fryer fish fingers recipe Good Food<br>BBCGoodFood/Air-fryer meatballs recipe Good Food |
| bbcgoodfood-02 | 28 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+iframes+links+jsonLd | BBCGoodFood/Air fryer Bramley apple crumble recipe Good Food<br>BBCGoodFood/Apple & blackberry crumble recipe Good Food<br>BBCGoodFood/Apple crisp recipe Good Food |
| bbcgoodfood-03 | 7 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+instructionLists+duplicateInstructionRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links+jsonLd | BBCGoodFood/Brilliant Banana Loaf<br>BBCGoodFood/Chewy Chocolate Chip Cookies<br>BBCGoodFood/Easy White Bread |
| bbcgoodfood-04 | 1 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+unitControls+notes+equipment+images+remoteResources+scripts+styles+iframes+links+jsonLd | BBCGoodFood/Cheesy black bean quesadillas recipe Good Food |
| bbcgoodfood-05 | 1 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+instructionLists+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links+jsonLd | BBCGoodFood/Choc-orange energy boosters recipe Good Food |

### BonAppetit

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| bonappetit-01 | 76 | recipeHeading+ingredientRegion+instructionRegion+instructionLists+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | BonAppetit/Anything-Goes Donabe<br>BonAppetit/BA's Best Risotto<br>BonAppetit/Baked Sweet Potato Chaat Recipe |
| bonappetit-02 | 17 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | BonAppetit/BA's Best Lasagna<br>BonAppetit/Boozy Cherry and Chocolate Pavlova Recipe<br>BonAppetit/Chocolate Guinness Cake Recipe |
| bonappetit-03 | 4 | recipeHeading+ingredientRegion+instructionRegion+instructionLists+formControls+notes+images+remoteResources+scripts+styles+iframes+links | BonAppetit/Artichoke Beurre Recipe<br>BonAppetit/Homemade Tartar Sauce Recipe<br>BonAppetit/Quick-Pickled Red Onions Recipe |
| bonappetit-04 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+iframes+links | BonAppetit/Banana Pudding Paris-Brest Recipe<br>BonAppetit/Cheddar Biscuits With Old Bay Recipe |
| bonappetit-05 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | BonAppetit/2-Ingredient Chocolate Mousse Recipe |

### BudgetBytes

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| budgetbytes-01 | 51 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/BBQ Chicken Foil Packs<br>BudgetBytes/Baked Pasta with Spring Veggies<br>BudgetBytes/Blueberry Cookies |
| budgetbytes-02 | 24 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/Bibimbap (Korean Mixed Rice Bowl)<br>BudgetBytes/Caesar Slaw with Crispy Chicken<br>BudgetBytes/Chicken Kabobs |
| budgetbytes-03 | 12 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/Chicken Noodle Soup (from scratch)<br>BudgetBytes/Classic Chili Recipe {So Easy!}<br>BudgetBytes/Cowboy Caviar Recipe |
| budgetbytes-04 | 4 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/Chicken Enchilada Soup<br>BudgetBytes/Easy Homemade Meatballs<br>BudgetBytes/Homemade Meatballs |
| budgetbytes-05 | 4 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/Chimichurri Grilled Chicken<br>BudgetBytes/Cold Sesame Noodles<br>BudgetBytes/Crockpot Beef Stew |
| budgetbytes-06 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+yieldControls+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/Easy Minestrone Soup<br>BudgetBytes/How to Make the Best Veggie Sandwich |
| budgetbytes-07 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+duplicateInstructionRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/Breakfast Pizza |
| budgetbytes-08 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+yieldControls+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/Viral 15-Minute Vegan Creamy Mushroom Ramen |
| budgetbytes-09 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+yieldControls+images+remoteResources+scripts+styles+eventHandlers+iframes+links | BudgetBytes/Balsamic Roasted Mushrooms with Herby Kale Mashed Potatoes |

### Epicurious

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| epicurious-01 | 70 | recipeHeading+ingredientRegion+instructionRegion+instructionLists+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | Epicurious/Baked Feta Pasta With Green Sauce Recipe<br>Epicurious/Berries and Cream Mille-Feuille Recipe<br>Epicurious/Big Bean Ceviche Recipe |
| epicurious-02 | 19 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | Epicurious/A Better Beer Can Chicken Recipe<br>Epicurious/BA's Best Carrot Cake<br>Epicurious/BA's Best Morning Buns Recipe |
| epicurious-03 | 7 | recipeHeading+ingredientRegion+instructionRegion+instructionLists+formControls+notes+images+remoteResources+scripts+styles+iframes+links | Epicurious/Artichoke Beurre Recipe<br>Epicurious/Berry Protein Cottage Cheese Smoothie Recipe<br>Epicurious/Dirty Martini Spritz Recipe |
| epicurious-04 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+iframes+links | Epicurious/Korean-Style Stuffed Peppers Recipe<br>Epicurious/Lamington Cupcakes Recipe |
| epicurious-05 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | Epicurious/2-Ingredient Chocolate Mousse Recipe |
| epicurious-06 | 1 | recipeHeading+ingredientRegion+instructionRegion+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | Epicurious/Extra-Smooth Lemon Curd Recipe |

### Food.com

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| food-com-01 | 81 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+unitControls+yieldControls+images+remoteResources+scripts+styles+links+jsonLd | Food.com/! Arriba ! Baked Winter Squash Mexican-Style Recipe<br>Food.com/# 1 Baked Chicken With Marinara Sauce Recipe<br>Food.com/#1 Asian Noodle Salad Recipe |
| food-com-02 | 9 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+unitControls+yieldControls+images+remoteResources+scripts+styles+iframes+links+jsonLd | Food.com/Apple Crisp<br>Food.com/Basic Pasta Dough<br>Food.com/Buttermilk Rolls or Classic White Bread |
| food-com-03 | 8 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+unitControls+yieldControls+images+remoteResources+scripts+styles+links+jsonLd | Food.com/#1 Best Vegan--Style Granola in the World! Recipe<br>Food.com/#1 Favorite Chinese Steamed Whole Fish by Sy Recipe - Chinese.Food.com<br>Food.com/#1 Favorite Mexican Casserole Recipe |
| food-com-04 | 1 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+unitControls+yieldControls+images+remoteResources+scripts+styles+iframes+links+jsonLd | Food.com/Flaky Pie Crust |
| food-com-05 | 1 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+unitControls+yieldControls+images+remoteResources+scripts+styles+links+jsonLd | Food.com/0 Point Ice Cream- Only 1 Ingredient Recipe |

### Food52

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| food52-01 | 66 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+iframes+links | Food52/Air-Fryer Chile-Cheese Toast<br>Food52/Air-Fryer Korean-Style Beef Tacos<br>Food52/Balsamic Vinaigrette-Marinated Skirt Steak With Tomato Salad |
| food52-02 | 17 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+iframes+links | Food52/Avocado Deviled Eggs with Smoked Salmon and Fried Capers<br>Food52/Cajun-Spiced Skirt Steak From Matt Fisher<br>Food52/Calabrian Chili Chicken Wings |
| food52-03 | 17 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+iframes+links | Food52/Air-Fryer Gochujang Chicken Wings<br>Food52/Chocolate Peanut Butter Cookies (aka Magic Middles)<br>Food52/Grilled Steak Sandwich with Blue Cheese Coleslaw |

### FoodNetwork

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| foodnetwork-01 | 43 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/10 Minute Apple Sauce Recipe Alton Brown<br>FoodNetwork/10 Minute Apple Sauce Recipe Alton Brown [44f41839]<br>FoodNetwork/10 Minute Fruit Tart Recipe Rachael Ray |
| foodnetwork-02 | 17 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/1-2-3 Lasagna Recipe<br>FoodNetwork/10,000 Island Dressing with Layered Salad Recipe Guy Fieri<br>FoodNetwork/10-inch French Meat Pie Recipe |
| foodnetwork-03 | 13 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/1 Dish Salmon + Veggies Recipe Molly Yeh<br>FoodNetwork/1 S'more for the Road and Kiddie S'mores Recipe Jeff Mauro<br>FoodNetwork/1-Hour Beef Pho Recipe |
| foodnetwork-04 | 12 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/10-Minute Cracker Pumpkin Pie Recipe<br>FoodNetwork/10-Minute Ice Cream Pops Recipe<br>FoodNetwork/15-Minute Bean, Egg and Avocado Tostadas Recipe |
| foodnetwork-05 | 5 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+duplicateIngredientRegion+formControls+yieldControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/15-Minute Stir-Fried Steak Tacos Recipe<br>FoodNetwork/2-Ingredient Chocolate Mousse Recipe<br>FoodNetwork/20-Minute Instant Pot Pizza Frittata Recipe |
| foodnetwork-06 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/10-minute Salmon, Couscous and Summer Zucchini Noodle Salad Recipe<br>FoodNetwork/15-Hour Potatoes with a Cool Sour Cream and Chive Dipping Sauce Recipe Jeff Mauro |
| foodnetwork-07 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/11-Can Soup and Cheesy Toast Recipe Ree Drummond<br>FoodNetwork/24 Karrot Gold Cupcakes Recipe |
| foodnetwork-08 | 2 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/1-Step Chipotle Hummus Recipe<br>FoodNetwork/2-Step Potato-Chorizo Hors d'Oeuvres Recipe |
| foodnetwork-09 | 2 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+iframes+links+jsonLd | FoodNetwork/Basic Pizza Dough<br>FoodNetwork/The Best Banana Bread |
| foodnetwork-10 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+duplicateIngredientRegion+formControls+yieldControls+notes+equipment+images+remoteResources+scripts+styles+links | FoodNetwork/16-Minute Greek Feast To Go Recipe Ree Drummond |
| foodnetwork-11 | 1 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+iframes+links+jsonLd | FoodNetwork/Chicken Parmesan |

### KingArthurBaking

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| kingarthurbaking-01 | 81 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+images+remoteResources+scripts+styles+iframes+links | KingArthurBaking/100% Whole Wheat Hamburger Buns Recipe<br>KingArthurBaking/American Flag Pie Recipe<br>KingArthurBaking/BLT Pizza Recipe |
| kingarthurbaking-02 | 15 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | KingArthurBaking/Apple Cinnamon Sourdough Bread Recipe<br>KingArthurBaking/Blueberry Shortcakes with Lime and Ginger Recipe<br>KingArthurBaking/Chocolate Icebox Cake Recipe |
| kingarthurbaking-03 | 4 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | KingArthurBaking/Brown Sugar Strawberry Frosted Cake Recipe<br>KingArthurBaking/Chewy Lemon Blondies Recipe<br>KingArthurBaking/Chocolate Chip Cookie Cake Recipe |

### NYTCooking

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| nytcooking-01 | 93 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+images+remoteResources+scripts+styles+links | NYTCooking/Baked Ziti<br>NYTCooking/Beef Bourguignon Recipe<br>NYTCooking/Best Chocolate Chip Cookies Recipe |
| nytcooking-02 | 2 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+images+remoteResources+scripts+styles+iframes+links | NYTCooking/Shakshuka with Feta<br>NYTCooking/Thai-Inspired Chicken Meatball Soup |
| nytcooking-03 | 2 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+formControls+images+remoteResources+scripts+styles+links | NYTCooking/Cranberry Curd Tart Recipe<br>NYTCooking/Creamy Cauliflower Soup With Rosemary Olive Oil Recipe |
| nytcooking-04 | 1 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+images+remoteResources+scripts+styles+links | NYTCooking/Five-Ingredient Creamy Miso Pasta Recipe |
| nytcooking-05 | 1 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+images+remoteResources+scripts+styles+iframes+links | NYTCooking/No-Knead Bread |
| nytcooking-06 | 1 | schemaOrgRecipe+recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+images+remoteResources+scripts+styles+links | NYTCooking/No-Knead Bread Recipe |

### RecipeTinEats

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| recipetineats-01 | 20 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+duplicateIngredientRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/A Very Big Pumpkin Layer Cake - with Toffee Pecans and Cream Cheese Frosting<br>RecipeTinEats/Broccoli Wedge Salad<br>RecipeTinEats/Chinese Orange Chicken |
| recipetineats-02 | 20 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+duplicateIngredientRegion+formControls+unitControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/15 Minute Sausage Meatballs<br>RecipeTinEats/Almond Lace Tuiles<br>RecipeTinEats/Beurre Blanc Sauce |
| recipetineats-03 | 6 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Chicken and Sweet Corn Soup with Noodles<br>RecipeTinEats/Coffee Cake with Cinnamon Streusel<br>RecipeTinEats/Country Pork Meatballs with Onion Gravy |
| recipetineats-04 | 4 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+unitControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Coquilles Saint-Jacques<br>RecipeTinEats/French Pork Schnitzel<br>RecipeTinEats/JB's Banana Bread |
| recipetineats-05 | 3 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Beef Rice Noodles<br>RecipeTinEats/Easy Chinese Beef Stir Fry - New & improved!<br>RecipeTinEats/Thai glass noodles (Pad Woon Sen) |
| recipetineats-06 | 3 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+duplicateIngredientRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Birthday Cake!<br>RecipeTinEats/Slow Cooker Beef Stew<br>RecipeTinEats/Tres Leches Cake |
| recipetineats-07 | 3 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+duplicateIngredientRegion+formControls+unitControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Golden Saffron Rice Bake with Baharat Spiced Meat<br>RecipeTinEats/Lamb Navarin<br>RecipeTinEats/Mushroom Ragu Pasta |
| recipetineats-08 | 3 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+duplicateIngredientRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Cajun Baked Salmon Bites<br>RecipeTinEats/Green Ginger Rice Noodles<br>RecipeTinEats/Taiwanese Three Cup Chicken |
| recipetineats-09 | 3 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Herbes de Provence<br>RecipeTinEats/Lemon curd<br>RecipeTinEats/Thai Dipping Sauce - Nam Jim Jaew |
| recipetineats-10 | 2 | recipeHeading+formControls+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Christmas Cooking Hotline 2025!<br>RecipeTinEats/☀️ What I make when it's too hot to cook |
| recipetineats-11 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+formControls+unitControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/50 Clove Garlic Soup<br>RecipeTinEats/Thai Red Lentil Soup |
| recipetineats-12 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Nagi's Salad Sandwich<br>RecipeTinEats/Spanakopita |
| recipetineats-13 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Creamy Jalapeño Sauce<br>RecipeTinEats/Instant Pink Taco Sauce |
| recipetineats-14 | 2 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Biscotti<br>RecipeTinEats/Choc Chip Peanut Butter Cookies |
| recipetineats-15 | 2 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Chimichurri Butter<br>RecipeTinEats/Creamy French Dressing |
| recipetineats-16 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+duplicateIngredientRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Vietnamese Pho |
| recipetineats-17 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Moussaka |
| recipetineats-18 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/12 Minute Baked Salmon Fajita Spread |
| recipetineats-19 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Baby Gem Lettuce Salad |
| recipetineats-20 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Rump Cap Roast (Picanha) |
| recipetineats-21 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/One Pot Lasagna - with layers! |
| recipetineats-22 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Raspberry coulis |
| recipetineats-23 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Thai Chicken (Gai Yang) |
| recipetineats-24 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Greek Lemon Potatoes |
| recipetineats-25 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+unitControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Naan |
| recipetineats-26 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Beef Stroganoff |
| recipetineats-27 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/🌈 Rainbow Chicken Stir Fry 🌈 |
| recipetineats-28 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Salted Caramel Cheesecake |
| recipetineats-29 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateIngredientRegion+formControls+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Baharat spice blend (Lebanese 7 Spice) |
| recipetineats-30 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Pastel de Choclo (Chilean Corn and Beef Casserole) |
| recipetineats-31 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Classic Baked Cheesecake |
| recipetineats-32 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateInstructionRegion+formControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Butter Chicken |
| recipetineats-33 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Thai Red Curry with Chicken |
| recipetineats-34 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Roasted Garlic Butter |
| recipetineats-35 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+duplicateIngredientRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Single serve chocolate chip cookie |
| recipetineats-36 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | RecipeTinEats/Fresh Basil Vinaigrette |
| recipetineats-37 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+unitControls+yieldControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Cheese Soufflé - Soufflé au Fromage |
| recipetineats-38 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+unitControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | RecipeTinEats/Tzatziki |

### SallysBakingAddiction

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| sallysbakingaddiction-01 | 34 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/9x13-Inch Pan Brownie Recipe - Sally's Baking<br>SallysBakingAddiction/Apple Cinnamon Rolls - Sally's Baking<br>SallysBakingAddiction/Baked Cinnamon Sugar Donuts - Sally's Baking |
| sallysbakingaddiction-02 | 21 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/Almond Flour Sugar Cookies (GF) - Sally's Baking<br>SallysBakingAddiction/Apple Brie Phyllo Galette - Sally's Baking<br>SallysBakingAddiction/Big Fat Dark Chocolate Cranberry Oatmeal Cookies - Sally's Baking |
| sallysbakingaddiction-03 | 10 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SallysBakingAddiction/Applesauce Muffins Recipe - Sally's Baking<br>SallysBakingAddiction/Bacon Wrapped Cheesy Stuffed Jalapeños - Sally's Baking<br>SallysBakingAddiction/Best Chocolate Chip Cookies (Popular Recipe!) - Sally's Baking |
| sallysbakingaddiction-04 | 7 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SallysBakingAddiction/Best Banana Bread<br>SallysBakingAddiction/Chocolate Layer Cake (Popular Recipe!) - Sally's Baking<br>SallysBakingAddiction/Homemade Candied Pecans - Sally's Baking |
| sallysbakingaddiction-05 | 4 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+yieldControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/Easy Coconut Shrimp - Sally's Baking<br>SallysBakingAddiction/Garlic Chicken & Spinach Stuffed Shells - Sally's Baking<br>SallysBakingAddiction/Healthy Citrus Chicken Quinoa Salad (Great for Meal Prep) - Sally's Baking |
| sallysbakingaddiction-06 | 4 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/20+ Sweet & Savory Zucchini Recipes - Sally's Baking<br>SallysBakingAddiction/25 Easy BBQ Desserts for Summer Cookouts - Sally's Baking<br>SallysBakingAddiction/Brown Butter Chocolate Chip Cookies - Sally's Baking |
| sallysbakingaddiction-07 | 3 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/Blueberry Crumble Pie - Sally's Baking<br>SallysBakingAddiction/Classic Chiffon Cake - Sally's Baking<br>SallysBakingAddiction/Meringue Roulade - Sally's Baking |
| sallysbakingaddiction-08 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/30 Recipes for Memorial Day - Sally's Baking<br>SallysBakingAddiction/Ultimate Snickers Cupcakes - Sally's Baking |
| sallysbakingaddiction-09 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/Make-Ahead Gingerbread Cinnamon Rolls - Sally's Baking<br>SallysBakingAddiction/Soft Chocolate Orange Sandwich Cookies (Like Milano Cookies) - Sally's Baking |
| sallysbakingaddiction-10 | 2 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/How to Freeze Bananas for Baking - Sally's Baking<br>SallysBakingAddiction/How to Make Linzer Cookies - Sally's Baking |
| sallysbakingaddiction-11 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SallysBakingAddiction/Guinness Brownies - Sally's Baking |
| sallysbakingaddiction-12 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SallysBakingAddiction/Homemade Blueberry Bagels (Real Flavor) - Sally's Baking |
| sallysbakingaddiction-13 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/Bread Pudding with Butter Rum Sauce - Sally's Baking |
| sallysbakingaddiction-14 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SallysBakingAddiction/The Best Sugar Cookies (Recipe & Video) - Sally's Baking |
| sallysbakingaddiction-15 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SallysBakingAddiction/Homemade Cake Pops - Sally's Baking |
| sallysbakingaddiction-16 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/Easy Blueberry French Toast Casserole - Sally's Baking |
| sallysbakingaddiction-17 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/Creamy Lemon Pie (7 Ingredients) - Sally's Baking |
| sallysbakingaddiction-18 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SallysBakingAddiction/Seriously Fudgy Homemade Brownies - Sally's Baking |
| sallysbakingaddiction-19 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/How to Make Baked Sweet Potato Fries - Sally's Baking |
| sallysbakingaddiction-20 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+links | SallysBakingAddiction/Gingerbread Blossoms - Sally's Baking |
| sallysbakingaddiction-21 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SallysBakingAddiction/Homemade Whipped Cream - Sally's Baking |

### SeriousEats

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| seriouseats-01 | 46 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+equipment+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Asparagus Quiche Recipe<br>SeriousEats/Balti-Style Eggs (Hard-Boiled Eggs in Coconut Milk Sauce)<br>SeriousEats/Besan Chilla (Savory Indian Chickpea Pancake) |
| seriouseats-02 | 30 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+equipment+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/30-Minute Harissa-Honey Salmon Rice Bowl Recipe<br>SeriousEats/Apple Pie Overnight Oats<br>SeriousEats/Banana–Nut Muffins Recipe |
| seriouseats-03 | 5 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Date Salad With Spicy Tahini-Yogurt Dressing<br>SeriousEats/Egg Drop Corn Soup (玉米羹) Recipe<br>SeriousEats/Hong Kong-Style Spam and Egg Sandwich Recipe |
| seriouseats-04 | 4 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+equipment+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Cheesy Sausage Breakfast Casserole Recipe<br>SeriousEats/Crème Caramel With Saffron, Rose and Cardamom Recipe<br>SeriousEats/Sfenj (Moroccan Doughnuts) |
| seriouseats-05 | 4 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Classic Balsamic Vinaigrette<br>SeriousEats/Spicy Sour Noodles (Inspired by Suan La Fen) Recipe<br>SeriousEats/Spicy Sour Noodles (Inspired by Suan La Fen) Recipe [8b833b09] |
| seriouseats-06 | 3 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Shrimp With Chorizo-Tomato Sauce Recipe<br>SeriousEats/Skillet Biscuits and Gravy Recipe<br>SeriousEats/Smoky Shrimp With Chorizo-Tomato Sauce Recipe |
| seriouseats-07 | 2 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Homemade Cream-Filled Doughnuts Recipe<br>SeriousEats/Summer Shrimp Pasta With Artichokes, Cherry Tomatoes, and Crispy Breadcrumbs Recipe |
| seriouseats-08 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+equipment+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Croissants Recipe |
| seriouseats-09 | 1 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Scottish-Style Oatmeal |
| seriouseats-10 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+notes+equipment+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SeriousEats/Foolproof Pan Pizza |
| seriouseats-11 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+equipment+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SeriousEats/All-American Beef Stew |
| seriouseats-12 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+equipment+images+remoteResources+styles+iframes+links | SeriousEats/Bravetart Chocolate Chip Cookies |
| seriouseats-13 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+unitControls+equipment+images+remoteResources+styles+eventHandlers+iframes+links | SeriousEats/Southwest-Style Cheesy Breakfast Strata |

### SimplyRecipes

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| simplyrecipes-01 | 30 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/Amish Macaroni Salad Recipe (Make Ahead)<br>SimplyRecipes/Banana Bread Breakfast Casserole Recipe (Make-Ahead)<br>SimplyRecipes/Caramelized Spam Mac and Cheese Recipe (One Pot, 35 Minutes) |
| simplyrecipes-02 | 18 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/Apple Cheddar Oat Cookies Recipe (30 Minutes)<br>SimplyRecipes/BLT Carbonara Recipe<br>SimplyRecipes/Cheesy Apple and Sausage Breakfast Casserole Recipe |
| simplyrecipes-03 | 18 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/Broccoli Pesto Pasta Recipe (20 Minutes)<br>SimplyRecipes/Caesar Potato Salad Recipe (4 Ingredients)<br>SimplyRecipes/Coronation Chicken Salad Recipe (5 Ingredients, 10 Minutes, Make-Ahead) |
| simplyrecipes-04 | 11 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/2-Ingredient Banana Pancakes Recipe<br>SimplyRecipes/Caramel Apple Overnight Oats Recipe (5 Ingredients, 10-Minute Prep, Make Ahead)<br>SimplyRecipes/Cold Soba Noodles Recipe (2 Ingredients, 10 Minutes) |
| simplyrecipes-05 | 10 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/Cheesy Chicken Alfredo Pasta Bake Recipe (4 Ingredients, 40 Minutes)<br>SimplyRecipes/Dad’s Shortcut Cookies & Cream Ice Cream Recipe (3 Ingredients)<br>SimplyRecipes/Easy Coddled Eggs Recipe (3 Ingredients, 20 Minutes) |
| simplyrecipes-06 | 7 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/Amish Broccoli Salad Recipe (25 Minutes)<br>SimplyRecipes/Food Processor Zucchini Bread Recipe<br>SimplyRecipes/Peanut Butter Chocolate Chip Banana Muffins Recipe (High-Protein, Gluten-Free, One-Bowl) |
| simplyrecipes-07 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+duplicateInstructionRegion+formControls+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/Coconut Creamed Corn Recipe (3 Ingredients, 20 Minutes) |
| simplyrecipes-08 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SimplyRecipes/Banana Bread |
| simplyrecipes-09 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SimplyRecipes/Chicken Pot Pie |
| simplyrecipes-10 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/Baked Feta Orzo Recipe (One Pan) |
| simplyrecipes-11 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateInstructionRegion+formControls+notes+images+remoteResources+styles+eventHandlers+iframes+links | SimplyRecipes/Honey-Chipotle Chicken Crispers Recipe (Chili’s Copycat, 30 Minutes) |
| simplyrecipes-12 | 1 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+formControls+notes+images+remoteResources+scripts+styles+eventHandlers+iframes+links | SimplyRecipes/Old-Fashioned Apple Pie |

### Tasty

| Cluster | Samples | Structural signature | Representative sample IDs |
| --- | ---: | --- | --- |
| tasty-01 | 85 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+links | Tasty/2-Ingredient Frozen Mango And Pineapple Ice Cream<br>Tasty/3 Fruity Summer Refreshers (Blended Fruit Drinks)<br>Tasty/50-Clove Garlic And Cheese Layer Potatoes |
| tasty-02 | 13 | recipeHeading+ingredientRegion+instructionRegion+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+links | Tasty/Easy Brazilian Lemonade Cocktail (Limonada Suíça)<br>Tasty/Easy Coffee Granita With Whipped Cream Cheese<br>Tasty/Easy Cowboy Caviar (Texas Caviar) |
| tasty-03 | 2 | recipeHeading+ingredientRegion+instructionRegion+groupedIngredients+ingredientLists+instructionLists+duplicateIngredientRegion+formControls+notes+images+remoteResources+scripts+styles+iframes+links | Tasty/One-Pot Garlic Parmesan Pasta<br>Tasty/The Best Chewy Chocolate Chip Cookies |

## Interpretation limits

- Feature detection is heuristic and operates on captured clipboard HTML; a count indicates structural evidence, not successful recipe extraction.
- Clusters group identical boolean feature signatures. They do not measure semantic equivalence, visual layout, or parser accuracy.
- Payload percentiles characterize captured files only and should inform, but not solely determine, parser resource limits.
- Recommended sample IDs are test-fixture candidates. Their recipe body content is deliberately excluded from this report and its JSON analysis.
