use crate::components::informational::PageMeta;
use dioxus::prelude::*;
const TOUR_CSS: Asset = asset!(
    "/assets/about-tour.css",
    CssAssetOptions::new().with_minify(false)
);
const PENGUINI_RIG: Asset = asset!("/assets/brand/penguini/still.svg");
const PENGUINI_LOADER: Asset = asset!(
    "/assets/about-penguini.js",
    JsAssetOptions::new().with_minify(false)
);
const PENGUINI_CLIPS: Asset = asset!(
    "/assets/brand/penguini/clips.js",
    JsAssetOptions::new().with_minify(false)
);
const PENGUINI_PLAYER: Asset = asset!(
    "/assets/brand/penguini/player.js",
    JsAssetOptions::new().with_minify(false)
);
const PENGUINI_THINKING: Asset = asset!(
    "/assets/brand/penguini/thinking-rig.js",
    JsAssetOptions::new().with_minify(false)
);
const PENGUINI_DANCE: Asset = asset!(
    "/assets/brand/penguini/dance-rig.js",
    JsAssetOptions::new().with_minify(false)
);
// Recompute asset fingerprints when only the animation scripts change.
const _: &[u8] = include_bytes!("../../assets/about-penguini.js");
const _: &[u8] = include_bytes!("../../assets/brand/penguini/clips.js");
const _: &[u8] = include_bytes!("../../assets/brand/penguini/player.js");
const _: &[u8] = include_bytes!("../../assets/brand/penguini/thinking-rig.js");
const _: &[u8] = include_bytes!("../../assets/brand/penguini/dance-rig.js");
const KITCHEN_MODEL: Asset = asset!("/assets/about/sams-kitchen-detailed.glb");
const KITCHEN_LAPTOP: Asset = asset!(
    "/assets/about/kitchen-laptop.webp",
    manganis::AssetOptions::builder()
);
const KITCHEN_COOKBOOK: Asset = asset!(
    "/assets/about/kitchen-cookbook.webp",
    manganis::AssetOptions::builder()
);
const KITCHEN_SURFACE_COLOR: Asset = asset!(
    "/assets/about/kitchen-surface-color.webp",
    manganis::AssetOptions::builder()
);
const KITCHEN_SURFACE_NORMAL: Asset = asset!(
    "/assets/about/kitchen-surface-normal.webp",
    manganis::AssetOptions::builder()
);
const KITCHEN_POSTER: Asset = asset!(
    "/assets/about/sams-kitchen.webp",
    manganis::AssetOptions::builder()
);
const LIBRARY_MODEL: Asset = asset!("/assets/about/sams-library.glb");
const LIBRARY_POSTER: Asset = asset!("/assets/about/sams-library.webp");
const SUPERMARKET_MODEL: Asset = asset!("/assets/about/sams-supermarket.glb");
const SUPERMARKET_POSTER: Asset = asset!("/assets/about/sams-supermarket.webp");
const CORPORATE_OFFICE_MODEL: Asset = asset!("/assets/about/sams-corporate-office.glb");
const CORPORATE_OFFICE_POSTER: Asset = asset!("/assets/about/sams-corporate-office.webp");
const OFFICE_MODEL: Asset = asset!("/assets/about/sams-office.glb");
const OFFICE_LEFT: Asset = asset!(
    "/assets/about/office-editor-left.webp",
    manganis::AssetOptions::builder()
);
const OFFICE_RIGHT: Asset = asset!(
    "/assets/about/office-editor-right.webp",
    manganis::AssetOptions::builder()
);
const OFFICE_GPU: Asset = asset!(
    "/assets/about/office-rtx4090.webp",
    manganis::AssetOptions::builder()
);
const OFFICE_CPU: Asset = asset!(
    "/assets/about/office-intel-core.webp",
    manganis::AssetOptions::builder()
);
const OFFICE_POSTER: Asset = asset!(
    "/assets/about/sams-office.webp",
    manganis::AssetOptions::builder()
);
const BEDROOM_MODEL: Asset = asset!("/assets/about/sams-bedroom.glb");
const BEDROOM_PRINTS: Asset = asset!("/assets/about/bedroom-posters.webp");
const BEDROOM_POSTER: Asset = asset!("/assets/about/sams-bedroom.webp");
const POTLUCK_MODEL: Asset = asset!("/assets/about/sams-potluck.glb");
const POTLUCK_WINDOW: Asset = asset!(
    "/assets/about/potluck-window.webp",
    manganis::AssetOptions::builder()
);
const POTLUCK_TV: Asset = asset!(
    "/assets/about/potluck-tv.webp",
    manganis::AssetOptions::builder()
);
const POTLUCK_PASTRY: Asset = asset!(
    "/assets/about/potluck-pastry.webp",
    manganis::AssetOptions::builder()
);
const POTLUCK_SALT: Asset = asset!(
    "/assets/about/potluck-chips-salt.webp",
    manganis::AssetOptions::builder()
);
const POTLUCK_BBQ: Asset = asset!(
    "/assets/about/potluck-chips-bbq.webp",
    manganis::AssetOptions::builder()
);
const POTLUCK_POSTER: Asset = asset!(
    "/assets/about/sams-potluck.webp",
    manganis::AssetOptions::builder()
);
const GARDEN_MODEL: Asset = asset!("/assets/about/sams-garden.glb");
const GARDEN_PASTRY: Asset = asset!(
    "/assets/about/garden-pastry.webp",
    manganis::AssetOptions::builder()
);
const GARDEN_POSTER: Asset = asset!(
    "/assets/about/sams-garden.webp",
    manganis::AssetOptions::builder()
);
pub(super) const LOADING_PHRASES: [&str; 100] = [
    "The cookies are baking…",
    "The steak is defrosting…",
    "The oven is preheating…",
    "Counting the chocolate chips…",
    "Giving the croissants a pep talk…",
    "Negotiating with the sourdough starter…",
    "Asking the coffee to wake up…",
    "Flipping an imaginary pancake…",
    "Teaching the noodles to untangle…",
    "Looking for the other oven mitt…",
    "Telling the onions it's okay to cry…",
    "Checking the fridge for plot twists…",
    "Convincing the butter to soften…",
    "Making room for one more cookie…",
    "Giving the whisk a spin class…",
    "Politely interrogating the garlic…",
    "Waiting for the kettle's solo…",
    "Finding the thyme to relax…",
    "Untangling the spaghetti situation…",
    "Asking the dough to rise and shine…",
    "Arranging a meeting with the meatballs…",
    "Helping the soup find its spoon…",
    "Putting tiny pajamas on the potatoes…",
    "Giving the toast a standing ovation…",
    "Checking the eggs for stage fright…",
    "Counting sheep made of marshmallows…",
    "Assembling the emergency snack kit…",
    "Dusting the flour off the flour…",
    "Asking the blender to use its inside voice…",
    "Teaching the mushrooms to share a room…",
    "Applying sunscreen to the tomatoes…",
    "Booking a spa day for the dumplings…",
    "Rehearsing the pancake flip…",
    "Finding the avocado's perfect moment…",
    "Explaining personal space to the fries…",
    "Interviewing a suspicious zucchini…",
    "Making sure the cheese feels grate…",
    "Waiting for the rice to stop gossiping…",
    "Tucking the lasagna into its layers…",
    "Giving the broccoli a new haircut…",
    "Trying to reason with a hot pepper…",
    "Checking the croissants' moon phases…",
    "Helping the gravy find its flow…",
    "Warming up the backup waffle…",
    "Asking the carrots to keep their crunch…",
    "Taking attendance in the spice drawer…",
    "Putting the peas in alphabetical order…",
    "Finding a lid for this particular chaos…",
    "Making a tiny hammock for the banana…",
    "Preparing the ceremonial first bite…",
    "Training the tongs to click twice…",
    "Sending the biscuits a warm invitation…",
    "Bringing the jam to band practice…",
    "Making sure the pasta has a bow tie…",
    "Reading the muffin's autobiography…",
    "Helping the cinnamon rolls unwind…",
    "Casting extras for the fruit salad…",
    "Checking the pizza's crust credentials…",
    "Finding the cookie that rolled away…",
    "Giving the lemon a motivational squeeze…",
    "Consulting the council of spatulas…",
    "Asking the burger to hold it together…",
    "Polishing the imaginary silverware…",
    "Giving the baguette a boarding pass…",
    "Explaining gravity to the soufflé…",
    "Checking the wok's dance schedule…",
    "Asking the sushi to roll with it…",
    "Keeping an eye on the sneaky croutons…",
    "Helping the cake find its good side…",
    "Rounding up the runaway sesame seeds…",
    "Giving the ravioli tiny name tags…",
    "Waiting for the pickles to finish arguing…",
    "Asking the custard to stay composed…",
    "Adding a pinch of unnecessary drama…",
    "Teaching the lettuce to romaine calm…",
    "Finding the lost shaker of paprika…",
    "Putting the kettle on hold…",
    "Checking the sourdough's mood ring…",
    "Untying the pretzels' shoelaces…",
    "Organizing a support group for leftovers…",
    "Asking the caramel to stop being clingy…",
    "Turning the mushrooms into fun guys…",
    "Giving the tacos a safety briefing…",
    "Checking the noodles for loose ends…",
    "Convincing the ice cream to play it cool…",
    "Making tiny snow angels in the flour…",
    "Helping the bread get its beauty rest…",
    "Recharging the electric jellybean…",
    "Inviting the lentils to a potluck…",
    "Teaching the ramen proper bowl manners…",
    "Making the marshmallows sign a toast…",
    "Asking the coffee beans to spill the tea…",
    "Building a fort out of breadsticks…",
    "Checking the pancakes for flat jokes…",
    "Giving the onion rings a wedding rehearsal…",
    "Negotiating visiting hours with the fridge…",
    "Asking the gnocchi to knock first…",
    "Helping the pudding find its wobble…",
    "Counting the holes in the Swiss cheese…",
    "Saving the last imaginary cookie…",
];
pub(super) const IDS: [&str; 12] = [
    "idea",
    "why",
    "creator",
    "cooking-frustrations",
    "my-own-solution",
    "job-search",
    "same-idea",
    "community",
    "accessible",
    "no-disruptions",
    "your-privacy",
    "gather",
];
pub(super) const MODEL_FOR_STOP: [usize; 12] = [0, 6, 2, 7, 1, 5, 2, 3, 3, 3, 3, 4];
pub(super) const NAMES: [&str; 12] = [
    "The idea",
    "The reason",
    "The creator",
    "Cooking frustrations",
    "My own solution",
    "The job search",
    "The same idea",
    "The philosophy",
    "Free and accessible",
    "No disruptions",
    "Your privacy",
    "Building a community",
];
const EYEBROWS: [&str; 12] = [
    "About Noms",
    "Why was Noms created?",
    "About the creator",
    "About the creator",
    "About the creator",
    "About the creator",
    "About the creator",
    "The Noms Philosophy",
    "The Noms Philosophy",
    "The Noms Philosophy",
    "The Noms Philosophy",
    "The Noms Philosophy",
];
const TITLES: [&str; 12] = [
    "The Ultimate Recipe Manager",
    "A side project that grew in to something larger.",
    "Hi, I'm Sam!",
    "The inconveniences of a cooking hobby.",
    "Building my own solution.",
    "A project to support my job search.",
    "The same fundamental idea.",
    "Just a simple and easy-to-use app.",
    "Free and accessible.",
    "Without the disruptions.",
    "Respecting your privacy.",
    "Building a community.",
];
const PARTS: [&str; 12] = [
    "01", "02", "1 of 5", "2 of 5", "3 of 5", "4 of 5", "5 of 5", "1 of 5", "2 of 5", "3 of 5",
    "4 of 5", "5 of 5",
];
const ORIGINAL_CONTENT: [&[&str];5] = [
&["Noms is designed to be the ultimate unified solution for managing all your recipes.", "With tools that help cooks save and share their signature recipes, iterate and develop new recipes, and organize and plan meals.", "Featuring an optimized search engine, recipe editor, ingredient-scaling calculator, cooking mode, a grocery organizer, and more."],
&["Noms started a simple side project made to be presented on a resume.", "Motivated by the desire to solve the ongoing issue of needing to constantly write and rewrite disorganized recipes on Google Docs.", "What started as a side project idea later became an actual solution to a real problem, and has since grown in to a suite of tools for the everyday cook and the larger cooking community."],
&["I'm a professional software engineer and casual computer science enthusiast from Vancouver, Canada who happens to enjoy cooking in his spare time.", "I enjoy experimenting with food, everything from sandwiches to pastas to breads, cakes, noodles, etc. And also beverages like coffee and tea.", "As a cooking hobbyist, some of the consistent inconveniences I had was organizing recipes that changed over time, keeping track of what changes worked and didn't work, finding out what I could cook with limited ingredients, and scaling recipes when certain ingredients had awkward proportions.", "I initially wondered if there existed an app that could solve these issues, and although some attempts seem to have been made, existing solutions consistently seemed to lack the user experience that I would consider ideal. As such, I set out to build my own solution.", "To be honest, it started out as a somewhat non-serious side project that was simply meant to support my ongoing job search at that time as software engineering job recruiters typically identify passion and interest through non-academic work particularly in inexperienced candidates.", "After securing a job I took a break from working on the project for about a year, but later decided to rewrite the app properly, but with the same fundamental idea.", "Nowadays I continue to support the project as it solves a genuine problem I have, and my hope is to be able to share the app with other people in the community."],
&["Noms puts the user experience first.", "One of the fundamental problems with modern software these days seems to be the constant disruptive designs that make it difficult to simply enjoy the tool as a solution to a problem.", "Ultimately, every project does need a pathway to sustainability if it hopes to scale beyond one user, but usability should never be sacrificed for short-term profitability.", "As such, Noms aims to keep key features free and accessible without a subscription or inconvenient restrictions that you have to pay to remove.", "It aims to avoid any disruptive or instrusive advertising or sponsorships that get in the way actually using the app.", "And it commits to respecting user privacy by only collecting data that is necessary to specific features, being transparent about what data is collected, never selling or sharing the data for any reason, and not using the data to manipulate users in to making purchases.", "Noms philosophy is that its primary purpose is to be a great product that people can enjoy using."],
&["Noms is a community oriented space.", "Food and cooking is a key part of how we build relationships and connections and Noms wants to be here for that.", "Come join us at (TBD)."]
];

const CONTENT: [&[&str]; 12] = [
    ORIGINAL_CONTENT[0],
    ORIGINAL_CONTENT[1],
    &[ORIGINAL_CONTENT[2][0], ORIGINAL_CONTENT[2][1]],
    &[ORIGINAL_CONTENT[2][2]],
    &[ORIGINAL_CONTENT[2][3]],
    &[ORIGINAL_CONTENT[2][4]],
    &[ORIGINAL_CONTENT[2][5], ORIGINAL_CONTENT[2][6]],
    &[ORIGINAL_CONTENT[3][0], ORIGINAL_CONTENT[3][1]],
    &[ORIGINAL_CONTENT[3][2], ORIGINAL_CONTENT[3][3]],
    &[ORIGINAL_CONTENT[3][4]],
    &[ORIGINAL_CONTENT[3][5], ORIGINAL_CONTENT[3][6]],
    ORIGINAL_CONTENT[4],
];

#[component]
fn LoadingPenguini() -> Element {
    use_drop(|| {
        let _ = document::eval("document.getElementById('tour-loader-penguini')?.dispatchEvent(new Event('noms-penguini-dispose'));");
    });
    rsx! {
        div {
            id: "tour-loader-penguini", class: "tour-loader-mascot", aria_hidden: "true",
            onmounted: move |_| {
                spawn(async move {
                    let assets = serde_json::json!({
                        "rig": PENGUINI_RIG.to_string(), "clips": PENGUINI_CLIPS.to_string(),
                        "player": PENGUINI_PLAYER.to_string(), "thinking": PENGUINI_THINKING.to_string(),
                        "dance": PENGUINI_DANCE.to_string(),
                    });
                    let module = serde_json::to_string(&PENGUINI_LOADER.to_string()).unwrap();
                    let script = format!("const host = document.getElementById('tour-loader-penguini'); const {{mountLoadingPenguini}} = await import({module}); if (host?.isConnected) mountLoadingPenguini(host, {assets});");
                    if let Err(error) = document::eval(&script).await {
                        tracing::warn!(?error, "About loader uses Penguini's resting pose");
                    }
                });
            },
            img { src: PENGUINI_RIG, alt: "", width: 1200, height: 1390, loading: "lazy", decoding: "async" }
        }
        button {
            class: "tour-loader-mascot-toggle", r#type: "button", hidden: true,
            "data-penguini-pause": "", aria_pressed: "false", "Pause animation"
        }
    }
}

#[component]
pub fn About() -> Element {
    #[cfg(target_arch = "wasm32")]
    use_drop(super::about_tour::stop);
    rsx! {
      PageMeta{title:"About Noms — The Ultimate Recipe Manager", description:"Meet Sam, learn why Noms was created, and explore the philosophy behind an easier way to save, develop and share recipes.",path:"/about"}
      document::Stylesheet{href:TOUR_CSS}
      div{id:"about-tour",class:"about-tour is-reader","data-kitchen-url":KITCHEN_MODEL,"data-kitchen-laptop-url":KITCHEN_LAPTOP,"data-kitchen-cookbook-url":KITCHEN_COOKBOOK,"data-kitchen-surface-color-url":KITCHEN_SURFACE_COLOR,"data-kitchen-surface-normal-url":KITCHEN_SURFACE_NORMAL,
       "data-library-url":LIBRARY_MODEL,"data-supermarket-url":SUPERMARKET_MODEL,"data-corporate-office-url":CORPORATE_OFFICE_MODEL,"data-office-url":OFFICE_MODEL,"data-office-left-url":OFFICE_LEFT,"data-office-right-url":OFFICE_RIGHT,"data-office-gpu-url":OFFICE_GPU,"data-office-cpu-url":OFFICE_CPU,
       "data-bedroom-url":BEDROOM_MODEL,"data-bedroom-prints-url":BEDROOM_PRINTS,
       "data-garden-url":GARDEN_MODEL,"data-garden-pastry-url":GARDEN_PASTRY,
       "data-potluck-url":POTLUCK_MODEL,"data-potluck-window-url":POTLUCK_WINDOW,"data-potluck-tv-url":POTLUCK_TV,"data-potluck-pastry-url":POTLUCK_PASTRY,"data-potluck-salt-url":POTLUCK_SALT,"data-potluck-bbq-url":POTLUCK_BBQ,
        onmounted: move |_| {
          #[cfg(target_arch="wasm32")]
          if let Err(error)=super::about_tour::start(){tracing::warn!(?error,"About tour uses its reading fallback");}
        },
     section{id:"tour-loader",class:"tour-loader",aria_labelledby:"tour-loader-title",
      div{class:"tour-loader-card",
       LoadingPenguini {}
       span{class:"eyebrow","ABOUT NOMS"}
       h2{id:"tour-loader-title","Setting the scene"}
       p{id:"tour-loading-status",aria_hidden:"true","{LOADING_PHRASES[0]}"}
       span{class:"sr-only",role:"status",aria_live:"polite","Loading the 3D story. The tour will open when it is ready."}
       div{class:"tour-loader-track",aria_hidden:"true",span{}}
       button{id:"tour-loader-read",r#type:"button","Click here if the page takes too long to load"}
      }
     }
     a{class:"skip-link",href:"#idea","Skip to the story"}
     header{id:"tour-header",class:"site-header",
      span{class:"tour-brand","About Noms"}
      span{class:"header-label","A JOURNEY THROUGH OUR STORY"}
      div{class:"header-controls",
       button{id:"motion-toggle",r#type:"button",disabled:true,aria_pressed:"false",aria_label:"Pause ambient motion",span{class:"pause-icon",aria_hidden:"true"}span{id:"motion-label","Pause drift"}}
       button{id:"mode-toggle",r#type:"button",disabled:true,aria_pressed:"false","Reading view"}
      }
     }
     div{id:"world",class:"world",aria_hidden:"true",canvas{id:"story-canvas",tabindex:"-1"}div{id:"tour-scene-framing"}div{class:"world-vignette"}}
     div{id:"story",class:"story",
      for i in 0..IDS.len() {
       article{id:"panel-{i}",class:"story-panel","data-panel":"{i}","data-model":"{MODEL_FOR_STOP[i]}",aria_labelledby:"title-{i}",
        span{id:"{IDS[i]}",class:"reader-anchor",aria_hidden:"true"}
        div{class:"panel-viewport",div{class:"panel-content",
        div{class:"panel-meta",span{class:"eyebrow","{EYEBROWS[i]}"}span{class:"panel-number","{PARTS[i]}"}}
        if i==0{h1{id:"title-{i}","{TITLES[i]}"}}else{h2{id:"title-{i}","{TITLES[i]}"}}
        if MODEL_FOR_STOP[i]==0 {figure{class:"kitchen-fallback",img{src:KITCHEN_POSTER,alt:"A kitchen for saving and developing recipes, with the Noms editor on a laptop and an open recipe book on the island",loading:"lazy",width:960,height:960}}}
        if MODEL_FOR_STOP[i]==1 {figure{class:"office-fallback",img{src:OFFICE_POSTER,alt:"Sam’s office with three code monitors, a mesh chair, a glass-sided PC, coffee equipment and custom keyboards",loading:"lazy",width:960,height:960}}}
        if MODEL_FOR_STOP[i]==2 {figure{class:"bedroom-fallback",img{src:BEDROOM_POSTER,alt:"Sam’s hobby bedroom with a custom keyboard workbench, badminton gear, collectible penguins and a cozy lounge",loading:"lazy",width:960,height:960}}}
        if MODEL_FOR_STOP[i]==3 {figure{class:"potluck-fallback",img{src:POTLUCK_POSTER,alt:"A communal potluck dining room and TV lounge with three couches, a chess game, and a table of cake, candy, chips and drinks",loading:"lazy",width:960,height:720}}}
        if MODEL_FOR_STOP[i]==6 {figure{class:"office-fallback",img{src:LIBRARY_POSTER,alt:"A recipe library with cookbook shelves, a reading table, laptop and cozy reading nook",loading:"lazy",width:1080,height:945}}}
        if MODEL_FOR_STOP[i]==7 {figure{class:"office-fallback",img{src:SUPERMARKET_POSTER,alt:"A modern supermarket with a European deli, Asian pantry, produce and grocery aisles",loading:"lazy",width:1073,height:960}}}
        if MODEL_FOR_STOP[i]==5 {figure{class:"office-fallback",img{src:CORPORATE_OFFICE_POSTER,alt:"A corporate office with three personalized desks, humorous company posters, a kitchenette and a shared lunch table",loading:"lazy",width:1080,height:960}}}
        if MODEL_FOR_STOP[i]==4 {figure{class:"garden-fallback",img{src:GARDEN_POSTER,alt:"A garden cookout with shared picnic tables, barbecue food, a beer table, lawn chairs, a cedar fence and outdoor games",loading:"lazy",width:960,height:720}}}
        for (j, paragraph) in CONTENT[i].iter().enumerate() {p{class:if j==0{"lead"}else{"detail"},"{paragraph}"}}

        div{class:"card-actions",
        if i>0 {a{class:"back-link",href:format!("#{}",IDS[i-1]),"Back"}}
        a{class:"next-link",href:if i+1<IDS.len(){format!("#{}",IDS[i+1])}else{"#idea".into()},
         if i==0{"Continue the story"}else if i+1==IDS.len(){"Take the tour again"}else{"Continue the story"}
         span{class:"arrow",aria_hidden:"true"}
        }}
        }}
       }
      }
     }
     div{id:"scroll-track",class:"scroll-track",aria_hidden:"true",
      for i in 0..IDS.len() {div{id:"stop-{i}",class:"waypoint","data-waypoint":"{i}"}}
     }
     span{id:"tour-chapter-status",class:"sr-only",role:"status",aria_live:"polite","{NAMES[0]}"}
     footer{class:"reader-footer",p{"Noms · Making great meals more accessible."}
      a{href:"/explore",class:"btn btn-secondary","Explore recipes"}
     }
      }
    }
}

#[cfg(test)]
mod story_tests {
    use super::*;
    #[test]
    fn cards_preserve_all_original_copy_and_unique_destinations() {
        assert_eq!(
            CONTENT.iter().flat_map(|p| p.iter()).collect::<Vec<_>>(),
            ORIGINAL_CONTENT
                .iter()
                .flat_map(|p| p.iter())
                .collect::<Vec<_>>()
        );
        assert_eq!(IDS.len(), 12);
        assert_eq!(
            IDS.iter().collect::<std::collections::HashSet<_>>().len(),
            12
        );
        assert_eq!(MODEL_FOR_STOP, [0, 6, 2, 7, 1, 5, 2, 3, 3, 3, 3, 4]);
        assert_eq!(CONTENT.len(), IDS.len());
    }
}
