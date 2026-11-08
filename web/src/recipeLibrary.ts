import { useRef, useState } from 'react';
import { readFile, RECIPES_PACKAGE, type CardChanges, type CardDirectory, type CardState } from './card';
import { recipeChange, type CardSession } from './cardSession';
import type { Language } from './i18n';
import { RECIPE_STRINGS } from './recipeStrings';
import {
  cardNeedsChoice,
  decodeLibrary,
  encodeLibrary,
  loadLibrary,
  packageLibraries,
  sameDial,
  sameRecipes,
  saveLibrary,
  shownDial,
  withIds,
  type Dial,
  type Family,
  type Recipe,
} from './recipes';

export interface RecipeLibrary {
  /** The library of the camera line of the page. */
  library: Recipe[];
  /** The recipes of the package: the library while the package has the Recipes module, else null. */
  packageRecipes: Recipe[] | null;
  /**
   * The recipes of a connected card for the camera line of the page that differ from its library,
   * while the person chooses which ones to keep; else null.
   */
  cardChoice: Recipe[] | null;
  /** Replaces the library of the camera line of the page. */
  change: (next: Recipe[]) => void;
  /** Ends the choice of `cardChoice`: `useCard` takes the recipes of the card. */
  chooseCard: (useCard: boolean) => void;
  /**
   * Takes the recipes of a card for the camera line `of`: at once while the library is empty or
   * the same, else after the person chooses (`cardChoice`), so a read of the card does not replace
   * the recipes of the site without a word.
   */
  adoptCard: (of: Family, recipes: readonly Recipe[]) => void;
  /** Drops a choice that waits for the recipes of an earlier card. */
  dropCardChoice: () => void;
  /**
   * Restores the recipe library of a package (`RECIPES_PACKAGE`) under `root`: it holds each recipe
   * of its camera lines, also those that are off, so it replaces the library of each line, as the
   * package gives the camera, the modules, and the film looks. Returns the count of recipes and the
   * camera line of the library when it has only one.
   */
  restorePackage: (root: CardDirectory) => Promise<{ count: number; family: Family | null }>;
  /**
   * The bytes of the recipe library of a ZIP package, or null without recipes. A recipe with the
   * automatic name of the camera keeps it, so INSTALL finds the recipe that the camera holds.
   */
  packageBytes: () => Promise<Uint8Array | null>;
  /**
   * What the user modes of the mode dial do for the card `session` after a write: the dial of the
   * card until the person changes it. A user mode keeps a recipe that is off, as the camera does.
   */
  dial: (session: CardSession) => Dial;
  /** What the user modes of the mode dial show for the card `session`, as the camera shows them. */
  shownDial: (session: CardSession) => Dial;
  /** Sets what user mode `u` (0 for U1) of the mode dial does for the card `session`. */
  changeDial: (session: CardSession, u: number, value: number) => void;
  /** The change of the recipe file of `session` that a write makes. */
  cardChange: (session: CardSession) => CardChanges['recipes'];
  /** The line of the change list for the change of the recipe file of `session`, or null. */
  changeLine: (session: CardSession, language: Language, label: string) => { label: string; text: string } | null;
}

/**
 * The recipe library of each camera line, which `storage` keeps; a card or a package with recipes
 * replaces the library of its camera line. `family` is the camera line of the page, and
 * `inPackage` tells that the package has the Recipes module. Each change of a library calls
 * `onChange`.
 */
export function useRecipeLibrary({
  family,
  inPackage,
  storage,
  onChange,
}: {
  family: Family;
  inPackage: boolean;
  storage: () => Storage | null;
  onChange: () => void;
}): RecipeLibrary {
  const [libraries, setLibraries] = useState<Record<Family, Recipe[]>>(() => ({
    gr3: loadLibrary(storage(), 'gr3'),
    gr4: loadLibrary(storage(), 'gr4'),
    gr4Monochrome: loadLibrary(storage(), 'gr4Monochrome'),
  }));
  // The libraries after the last change, for a task that reads them after an await.
  const librariesNow = useRef(libraries);
  // The recipes of a connected card that differ from the library of their camera line: the
  // person chooses which ones to keep.
  const [cardRecipes, setCardRecipes] = useState<{ family: Family; recipes: Recipe[] } | null>(null);
  // The mode dial that the person set for the card whose read gave `state`. Another read of the
  // card, such as the one after a write, gives the dial of the card again.
  const [dialEdit, setDialEdit] = useState<{ state: CardState; dial: Dial } | null>(null);
  const dialOf = (session: CardSession) => (dialEdit?.state === session.state ? dialEdit.dial : session.dial);

  const library = libraries[family];
  const packageRecipes = inPackage ? library : null;

  // Each change goes through here, so each recipe of a library has its own ID (`withIds`).
  function changeLibrary(of: Family, recipes: Recipe[]) {
    const next = withIds(recipes);
    librariesNow.current = { ...librariesNow.current, [of]: next };
    setLibraries(librariesNow.current);
    saveLibrary(storage(), of, next);
    onChange();
  }

  return {
    library,
    packageRecipes,
    cardChoice: cardRecipes?.family === family ? cardRecipes.recipes : null,
    change: (next) => changeLibrary(family, next),
    chooseCard(useCard) {
      if (useCard && cardRecipes) changeLibrary(cardRecipes.family, cardRecipes.recipes);
      setCardRecipes(null);
    },
    adoptCard(of, read) {
      const recipes = [...read];
      if (cardNeedsChoice(librariesNow.current[of], recipes)) setCardRecipes({ family: of, recipes });
      else {
        if (!sameRecipes(librariesNow.current[of], recipes)) changeLibrary(of, recipes);
        setCardRecipes(null);
      }
    },
    dropCardChoice: () => setCardRecipes(null),
    async restorePackage(root) {
      const bytes = await readFile(root, RECIPES_PACKAGE);
      if (!bytes) return { count: 0, family: null };
      const recipes = await decodeLibrary(bytes);
      const restored = packageLibraries(recipes);
      for (const [of, next] of Object.entries(restored) as [Family, Recipe[]][]) {
        changeLibrary(of, next);
      }
      const lines = Object.keys(restored) as Family[];
      return { count: recipes.length, family: lines.length === 1 ? lines[0]! : null };
    },
    packageBytes: () =>
      packageRecipes && packageRecipes.length > 0 ? encodeLibrary(packageRecipes) : Promise.resolve(null),
    dial: dialOf,
    shownDial: (session) => shownDial(dialOf(session), library),
    changeDial(session, u, value) {
      const dial = dialOf(session).map((item, other) => (other === u ? value : item));
      if (sameDial(dial, dialOf(session))) return;
      setDialEdit({ state: session.state, dial });
      onChange();
    },
    cardChange: (session) => recipeChange(session, packageRecipes, dialOf(session)),
    changeLine(session, language, label) {
      if (recipeChange(session, packageRecipes, dialOf(session)) === 'same') return null;
      const r = RECIPE_STRINGS[language];
      // A change of the mode dial alone keeps the recipes of the card.
      if (sameRecipes(session.recipes ?? [], library)) return { label, text: r.changeDial };
      return { label, text: library.length ? r.changeWrite(library.length) : r.changeRemove };
    },
  };
}
