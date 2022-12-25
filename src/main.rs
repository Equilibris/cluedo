#![feature(maybe_uninit_array_assume_init, maybe_uninit_uninit_array)]
use serde::{Deserialize, Serialize};
use std::io::Write;
// use std::mem::MaybeUninit;

macro_rules! inventory {
    ($($name:ident $id:ident : $($a:ident,$b:ident,)+;)+) => {
        $(
            paste::paste! {
                #[derive(Copy, Clone, Debug, Default, Deserialize, Serialize)]
                struct [<$name Inventory>] < T >{ $($a : T,)+ }

                impl <T> std::ops::Index<$name> for [<$name Inventory>] <T>{
                    type Output = T;
                    fn index(&self, index:$name) -> &Self::Output {
                        match index { $($name::$b => &self.$a,)+ }
                    }
                }

                impl <T> std::ops::IndexMut<$name> for [<$name Inventory>] <T>{
                    fn index_mut(&mut self, index:$name,) -> &mut Self::Output {
                        match index { $($name::$b => &mut self.$a,)+ }
                    }
                }
            }
            #[derive(Clone,Copy,PartialEq,Eq,Debug,Deserialize, Serialize)]
            enum $name { $($b,)+ }
            impl $name {
                fn from_str<'a>(v: &'a str) -> Option<$name> {
                    match v {
                        $(stringify!($b) => Some(Self::$b),)+

                        _ => None,
                    }
                }

                fn iter() -> impl Iterator<Item = Self> {
                    let v = vec![$(Self::$b,)+];

                    v.into_iter()
                }
            }
        )+
        #[derive(Clone, Debug, Deserialize, Serialize)]
        struct Interaction { from: usize, to: usize, $($id: $name,)+}
        impl Interaction {
            pub fn new(from: usize, to: usize, $($id: $name,)+) -> Self {
                Self {from,to,$($id,)+}
            }
        }
        paste::paste! {
            #[derive(Debug, Default, Clone, Deserialize, Serialize)]
            struct Inventory <T = bool> { $($id: [<$name Inventory>] <T>,)+ }

            $(
                impl <T> std::ops::Index<$name> for Inventory <T>{
                    type Output = T;
                    fn index(&self, index: $name) -> &Self::Output {
                        &self.$id[index]
                    }
                }

                impl <T> std::ops::IndexMut<$name> for Inventory <T> {
                    fn index_mut(&mut self, index: $name) -> &mut Self::Output {
                        &mut self.$id[index]
                    }
                }
            )+
        }
    };
}

inventory!(Weapon weapon:
           rope,Rope,
           candlestick,Candlestick,
           lead_pipe,LeadPipe,
           revolver,Revolver,
           spanner,Spanner,
           dagger,Dagger,;

Person person:
           green,Green,
           white,White,
           plum,Plum,
           scarlet,Scarlet,
           mustard,Mustard,
           peacock,Peacock,;

Place place:
           study,Study,
           dining_room,DiningRoom,
           ball_room,BallRoom,
           library,Library,
           hall,Hall,
           conservatory,Conservatory,
           kitchen,Kitchen,;);

// macro_rules! arr {
//     ($init:expr ; $len:expr) => {{
//         let mut vs: [MaybeUninit<_>; $len] = MaybeUninit::uninit_array();
//         for i in vs.iter_mut() {
//             *i = MaybeUninit::new($init);
//         }

//         unsafe { MaybeUninit::array_assume_init(vs) }
//     }};
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[repr(u8)]
enum State {
    #[default]
    Unknown,
    Has,
    HasNot,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Game {
    names_invs: Vec<(String, Inventory<State>)>,
    interactions: Vec<Interaction>,
}

impl Game {
    fn new(names: Vec<String>) -> Self {
        Self {
            names_invs: names.into_iter().map(|v| (v, Default::default())).collect(),
            interactions: Default::default(),
        }
    }

    #[inline(always)]
    fn players(&self) -> usize {
        self.names_invs.len()
    }

    fn name_to_index(&self, name: impl AsRef<str>) -> Option<usize> {
        for (index, val) in self.names_invs.iter().map(|(a, _)| a).enumerate() {
            if name.as_ref() == val {
                return Some(index);
            }
        }

        None
    }
    fn index_to_name(&self, val: usize) -> Option<&'_ str> {
        if val == 0 {
            Some("me")
        } else if val <= self.players() {
            Some(self.names_invs.get(val - 1).unwrap().0.as_str())
        } else {
            None
        }
    }

    fn others(&self, idx: usize) -> Vec<usize> {
        let mut v = vec![0; self.players() - 1];

        let mut above = false;

        for (index, v) in v.iter_mut().enumerate() {
            if index == idx {
                above = true;
            }
            *v = index + (above as usize);
        }
        v
    }

    fn between(&self, from: usize, to: usize) -> Option<impl Iterator<Item = usize>> {
        if from > self.players() || to > self.players() {
            return None;
        }

        Some(if from == to {
            ((from + 1)..self.players()).chain(0..from)
        } else if from > to {
            ((from + 1)..to).chain(0..0)
        } else {
            ((from + 1)..self.players()).chain(self.players()..to)
        })
    }

    fn add_interaction(&mut self, interaction: Interaction) {
        for i in Self::between(&self, interaction.from, interaction.to).unwrap() {
            self.weapon_mark_has_not(i, interaction.weapon);
            self.person_mark_has_not(i, interaction.person);
            self.place_mark_has_not(i, interaction.place);
        }
        self.interactions.push(interaction);
        self.conduct_elimination();
    }

    fn conduct_elimination(&mut self) {
        loop {
            let mut mod_count = 0;
            for Interaction {
                from,
                to,
                weapon,
                person,
                place,
            } in self.interactions.clone()
            {
                use State::*;
                if to != from {
                    mod_count += 1;
                    match (
                        self.names_invs.get(to).unwrap().1[weapon],
                        self.names_invs.get(to).unwrap().1[person],
                        self.names_invs.get(to).unwrap().1[place],
                    ) {
                        (Unknown, HasNot, HasNot) => self.add_weapon_to_inv(to, weapon),
                        (HasNot, Unknown, HasNot) => self.add_person_to_inv(to, person),
                        (HasNot, HasNot, Unknown) => self.add_place_to_inv(to, place),
                        _ => mod_count -= 1,
                    }
                }
            }
            if mod_count == 0 {
                break;
            }
        }
    }

    fn weapon_mark_has_not(&mut self, inv: usize, item: Weapon) {
        self.names_invs.get_mut(inv).unwrap().1[item] = State::HasNot
    }
    fn person_mark_has_not(&mut self, inv: usize, item: Person) {
        self.names_invs.get_mut(inv).unwrap().1[item] = State::HasNot
    }
    fn place_mark_has_not(&mut self, inv: usize, item: Place) {
        self.names_invs.get_mut(inv).unwrap().1[item] = State::HasNot
    }

    fn add_weapon_to_inv(&mut self, inv: usize, weapon: Weapon) {
        self.names_invs.get_mut(inv).unwrap().1[weapon] = State::Has;

        for i in Self::others(&self, inv) {
            self.names_invs.get_mut(i).unwrap().1[weapon] = State::HasNot;
        }
    }
    fn add_place_to_inv(&mut self, inv: usize, weapon: Place) {
        self.names_invs.get_mut(inv).unwrap().1[weapon] = State::Has;

        for i in Self::others(&self, inv) {
            self.names_invs.get_mut(i).unwrap().1[weapon] = State::HasNot;
        }
    }
    fn add_person_to_inv(&mut self, inv: usize, weapon: Person) {
        self.names_invs.get_mut(inv).unwrap().1[weapon] = State::Has;

        for i in Self::others(&self, inv) {
            self.names_invs.get_mut(i).unwrap().1[weapon] = State::HasNot;
        }
    }
    fn available_options(&self) -> (Vec<Weapon>, Vec<Person>, Vec<Place>) {
        let mut weapons = Vec::new();
        'a: for w in Weapon::iter() {
            for i in 0..self.players() {
                if self.names_invs.get(i).unwrap().1[w] == State::Has {
                    continue 'a;
                }
            }
            weapons.push(w);
        }

        let mut people = Vec::new();
        'a: for p in Person::iter() {
            for i in 0..self.players() {
                if self.names_invs.get(i).unwrap().1[p] == State::Has {
                    continue 'a;
                }
            }
            people.push(p);
        }
        let mut places = Vec::new();
        'a: for p in Place::iter() {
            for i in 0..self.players() {
                if self.names_invs.get(i).unwrap().1[p] == State::Has {
                    continue 'a;
                }
            }
            places.push(p);
        }

        (weapons, people, places)
    }
}

fn supply_facts(
    s: &mut String,
    writer: &mut std::io::Stdout,
    reader: &std::io::Stdin,
    game: &mut Game,
) {
    loop {
        s.clear();

        print!("Enter fact: ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        let v = s.trim();

        if v.is_empty() {
            break;
        }

        match (Weapon::from_str(v), Person::from_str(v), Place::from_str(v)) {
            (Some(v), None, None) => game.add_weapon_to_inv(0, v),
            (None, Some(v), None) => game.add_person_to_inv(0, v),
            (None, None, Some(v)) => game.add_place_to_inv(0, v),
            _ => println!("This does not match a known fact. Make sure it is in camel case"),
        }
    }
}

fn interaction(
    s: &mut String,
    writer: &mut std::io::Stdout,
    reader: &std::io::Stdin,
    game: &mut Game,
) {
    let from = loop {
        s.clear();

        print!("Start: ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match game.name_to_index(s.trim()) {
            Some(x) => break x,
            None => println!(
                "This person does not exist, please enter one of {:?}",
                game.names_invs.iter().map(|(a, _)| a).collect::<Vec<_>>()
            ),
        }
    };
    let to = loop {
        s.clear();

        print!("End:   ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match game.name_to_index(s.trim()) {
            Some(x) => break x,
            None => println!(
                "This person does not exist, please enter one of {:?}",
                game.names_invs.iter().map(|(a, _)| a).collect::<Vec<_>>()
            ),
        }
    };

    let weapon = loop {
        s.clear();

        print!("Weapon: ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match Weapon::from_str(s.trim()) {
            Some(x) => break x,
            None => println!("Given str is not a valid weapon",),
        }
    };
    let person = loop {
        s.clear();

        print!("Person: ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match Person::from_str(s.trim()) {
            Some(x) => break x,
            None => println!("Given str is not a valid weapon",),
        }
    };
    let place = loop {
        s.clear();

        print!("Place:  ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match Place::from_str(s.trim()) {
            Some(x) => break x,
            None => println!("Given str is not a valid weapon",),
        }
    };

    let interaction = Interaction::new(from, to, weapon, person, place);

    game.add_interaction(interaction);
}

fn query(s: &mut String, writer: &mut std::io::Stdout, reader: &std::io::Stdin, game: &mut Game) {
    let from = 0;
    let to = loop {
        s.clear();

        print!("End:   ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match game.name_to_index(s.trim()) {
            Some(x) => break x,
            None => println!(
                "This person does not exist, please enter one of {:?}",
                game.names_invs.iter().map(|(a, _)| a).collect::<Vec<_>>()
            ),
        }
    };

    let weapon = loop {
        s.clear();

        print!("Weapon: ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match Weapon::from_str(s.trim()) {
            Some(x) => break x,
            None => println!("Given str is not a valid weapon",),
        }
    };
    let person = loop {
        s.clear();

        print!("Person: ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match Person::from_str(s.trim()) {
            Some(x) => break x,
            None => println!("Given str is not a valid weapon",),
        }
    };
    let place = loop {
        s.clear();

        print!("Place:  ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match Place::from_str(s.trim()) {
            Some(x) => break x,
            None => println!("Given str is not a valid weapon",),
        }
    };

    loop {
        s.clear();

        print!("Resolved Item Class (weapon person place):  ");
        writer.flush().unwrap();

        reader.read_line(s).unwrap();

        match s.trim() {
            "weapon" => {
                game.add_weapon_to_inv(to, weapon);
                break;
            }
            "person" => {
                game.add_person_to_inv(to, person);
                break;
            }
            "place" => {
                game.add_place_to_inv(to, place);
                break;
            }
            _ => continue,
        }
    }
    let interaction = Interaction::new(from, to, weapon, person, place);
    game.add_interaction(interaction);
}

fn main() {
    let num_players = loop {
        print!("Num players: ");

        std::io::stdout().flush().unwrap();

        let reader = std::io::stdin();

        let mut num = String::new();

        reader.read_line(&mut num).expect("Failed to read number");

        let num: usize = match num.trim().parse() {
            Ok(v) => v,
            _ => continue,
        };

        break num;
    };

    let mut names = vec![String::new(); num_players];

    let reader = std::io::stdin();
    let mut writer = std::io::stdout();
    let mut iter = names.iter_mut();
    *iter.next().unwrap() = "Me".to_string();

    for (index, val) in iter.enumerate() {
        print!("Name opponent #{}: ", index + 1);
        writer.flush().unwrap();

        reader.read_line(val).unwrap();
        *val = val.trim().to_string();
    }

    let mut game = Game::new(names);

    for v in Weapon::iter() {
        game.weapon_mark_has_not(0, v)
    }
    for v in Place::iter() {
        game.place_mark_has_not(0, v)
    }
    for v in Person::iter() {
        game.person_mark_has_not(0, v)
    }

    // Get facts
    let mut s = String::new();
    supply_facts(&mut s, &mut writer, &reader, &mut game);

    loop {
        s.clear();

        let (weapons, people, places) = game.available_options();
        println!("Remaining weapons {:?}", weapons);
        println!("Remaining people  {:?}", people);
        println!("Remaining places  {:?}", places);

        print!("Select action (interaction query print facts): ");
        writer.flush().unwrap();

        reader.read_line(&mut s).unwrap();

        let v = s.trim();

        match v {
            "interaction" => interaction(&mut s, &mut writer, &reader, &mut game),
            "query" => query(&mut s, &mut writer, &reader, &mut game),
            "facts" => supply_facts(&mut s, &mut writer, &reader, &mut game),
            "print" => println!("{:#?}", game),
            "save" => toml::de::from_str()
            _ => continue,
        }
    }
}
