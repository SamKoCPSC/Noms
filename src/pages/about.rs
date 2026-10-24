use crate::components::informational::PageMeta;
use dioxus::prelude::*;
const TOUR_CSS: Asset = asset!(
    "/assets/about-tour.css",
    CssAssetOptions::new().with_minify(false)
);
const KITCHEN_MODEL: Asset = asset!("/assets/about/sams-kitchen-detailed.glb");
const KITCHEN_POSTER: Asset = asset!("/assets/about/sams-kitchen.webp");
const IDS: [&str; 4] = ["idea", "why", "creator", "community"];
const NAMES: [&str; 4] = ["The idea", "The reason", "The creator", "The philosophy"];
const EYEBROWS: [&str; 4] = [
    "About Noms",
    "Why was Noms created?",
    "About the creator",
    "The Noms Philosophy",
];
const TITLES: [&str; 4] = [
    "The Ultimate Recipe Manager",
    "A side project that grew in to something larger.",
    "Hi, I'm Sam!",
    "Just a simple and easy-to-use app.",
];
const CONTENT: [&[&str];4] = [
&["Noms is designed to be the ultimate unified solution for managing all your recipes.", "With tools that help cooks save and share their signature recipes, iterate and develop new recipes, and organize and plan meals.", "Featuring an optimized search engine, recipe editor, ingredient-scaling calculator, cooking mode, a grocery organizer, and more."],
&["Noms started a simple side project made to be presented on a resume.", "Motivated by the desire to solve the ongoing issue of needing to constantly write and rewrite disorganized recipes on Google Docs.", "What started as a side project idea later became an actual solution to a real problem, and has since grown in to a suite of tools for the everyday cook and the larger cooking community."],
&["I'm a professional software engineer and casual computer science enthusiast from Vancouver, Canada who happens to enjoy cooking in his spare time.", "I enjoy experimenting with food, everything from sandwiches to pastas to breads, cakes, noodles, etc. And also beverages like coffee and tea.", "As a cooking hobbyist, some of the consistent inconveniences I had was organizing recipes that changed over time, keeping track of what changes worked and didn't work, finding out what I could cook with limited ingredients, and scaling recipes when certain ingredients had awkward proportions.", "I initially wondered if there existed an app that could solve these issues, and although some attempts seem to have been made, existing solutions consistently seemed to lack the user experience that I would consider ideal. As such, I set out to build my own solution.", "To be honest, it started out as a somewhat non-serious side project that was simply meant to support my ongoing job search at that time as software engineering job recruiters typically identify passion and interest through non-academic work particularly in inexperienced candidates.", "After securing a job I took a break from working on the project for about a year, but later decided to rewrite the app properly, but with the same fundamental idea.", "Nowadays I continue to support the project as it solves a genuine problem I have, and my hope is to be able to share the app with other people in the community."],
&["Noms puts the user experience first.", "One of the fundamental problems with modern software these days seems to be the constant disruptive designs that make it difficult to simply enjoy the tool as a solution to a problem.", "Ultimately, every project does need a pathway to sustainability if it hopes to scale beyond one user, but usability should never be sacrificed for short-term profitability.", "As such, Noms aims to keep key features free and accessible without a subscription or inconvenient restrictions that you have to pay to remove.", "It aims to avoid any disruptive or instrusive advertising or sponsorships that get in the way actually using the app.", "And it commits to respecting user privacy by only collecting data that is necessary to specific features, being transparent about what data is collected, never selling or sharing the data for any reason, and not using the data to manipulate users in to making purchases.", "Noms philosophy is that its primary purpose is to be a great product that people can enjoy using."]
];

#[component]
pub fn About() -> Element {
    #[cfg(target_arch = "wasm32")]
    use_drop(super::about_tour::stop);
    rsx! {
      PageMeta{title:"About Noms — The Ultimate Recipe Manager", description:"Meet Sam, learn why Noms was created, and explore the philosophy behind an easier way to save, develop and share recipes.",path:"/about"}
      document::Stylesheet{href:TOUR_CSS}
      div{id:"about-tour",class:"about-tour is-reader","data-kitchen-url":KITCHEN_MODEL,
        onmounted: move |_| {
          #[cfg(target_arch="wasm32")]
          if let Err(error)=super::about_tour::start(){tracing::warn!(?error,"About tour uses its reading fallback");}
        },
     a{class:"skip-link",href:"#idea","Skip to the story"}
     header{class:"site-header",
      span{class:"tour-brand","About Noms"}
      span{class:"header-label","A JOURNEY THROUGH OUR STORY"}
      div{class:"header-controls",
       button{id:"motion-toggle",r#type:"button",disabled:true,aria_pressed:"false",aria_label:"Pause ambient motion",span{class:"pause-icon",aria_hidden:"true"}span{id:"motion-label","Pause drift"}}
       button{id:"mode-toggle",r#type:"button",disabled:true,aria_pressed:"false","Reading view"}
      }
     }
     div{id:"world",class:"world",aria_hidden:"true",canvas{id:"story-canvas",tabindex:"-1"}div{class:"world-vignette"}}
     div{class:"journey-caption",aria_hidden:"true",span{"FOLLOW THE THREAD"}i{}span{id:"chapter-count","01 / 04"}}
     div{id:"story",class:"story",
      for i in 0..4 {
       article{id:"panel-{i}",class:"story-panel","data-panel":"{i}",aria_labelledby:"title-{i}",
        span{id:"{IDS[i]}",class:"reader-anchor",aria_hidden:"true"}
        div{class:"panel-meta",span{class:"eyebrow","{EYEBROWS[i]}"}span{class:"panel-number","0{i+1}"}}
        if i==0{h1{id:"title-{i}","{TITLES[i]}"}}else{h2{id:"title-{i}","{TITLES[i]}"}}
        if i==2 {figure{class:"kitchen-fallback",img{src:KITCHEN_POSTER,alt:"Sam's kitchen, with coffee equipment, sourdough, pastries and a meal on the island",loading:"lazy",width:960,height:960}}}
        for (j, paragraph) in CONTENT[i].iter().enumerate() {p{class:if j==0{"lead"}else{"detail"},"{paragraph}"}}
        if i==3 {div{class:"values",span{"Key features free"}span{"No intrusive ads"}span{"Your data stays yours"}}}
        a{class:"next-link",href:if i<3{format!("#{}",IDS[i+1])}else{"#idea".into()},
         if i==0{"Begin the journey"}else if i==3{"Take the tour again"}else{"Continue the story"}
         span{class:"arrow",aria_hidden:"true"}
        }
       }
      }
     }
     div{id:"scroll-track",class:"scroll-track",aria_hidden:"true",
      for i in 0..4 {div{id:"stop-{i}",class:"waypoint","data-waypoint":"{i}"}}
     }
     nav{class:"tour-nav",aria_label:"Story chapters",
      div{class:"nav-top",span{class:"tour-word","OUR STORY"}span{id:"stop-name",aria_live:"polite","The idea"}span{class:"scroll-hint","Scroll to travel"}}
      div{class:"route",div{class:"route-line",span{id:"route-progress"}}
       for i in 0..4 {a{id:"nav-{i}",href:"#{IDS[i]}",aria_label:"Chapter {i+1}: {NAMES[i]}",aria_current:if i==0{"step"}else{"false"},span{class:"route-dot"}span{class:"route-label","data-short":["Idea","Why","Creator","Philosophy"][i],"{NAMES[i]}"}}}
      }
     }
     footer{class:"reader-footer",p{"Noms · Making great meals more accessible."}
      a{href:"/explore",class:"btn btn-secondary","Explore recipes"}
     }
      }
    }
}
