import type { Tags } from '../types/tags';
import { ensureTagsFile } from '../utils/ensure-files';

/**
 * Registry of valid tags and the per-category lists used for required-tag
 * checks. Produced once by {@link loadTagRegistry} and passed through to
 * per-file validation.
 */
export interface TagRegistry {
  validTags: Set<string>;
  requiredCategories: {
    component: string[];
    featureGroup: string[];
  };
}

/**
 * Load the tag registry from `spec/tags.json` (creating it if missing via
 * `ensureTagsFile`) and derive the flat set of valid tag names plus the
 * component / feature-group tag lists used for required-tag validation.
 *
 * @param cwd - Project root to resolve `spec/tags.json` against.
 * @returns A populated {@link TagRegistry}.
 */
export async function loadTagRegistry(cwd: string): Promise<TagRegistry> {
  const tagsData: Tags = await ensureTagsFile(cwd);

  const validTags = new Set<string>();
  const componentTags: string[] = [];
  const featureGroupTags: string[] = [];

  for (const category of tagsData.categories) {
    for (const tag of category.tags) {
      validTags.add(tag.name);

      if (category.name === 'Component Tags') {
        componentTags.push(tag.name);
      } else if (category.name === 'Feature Group Tags') {
        featureGroupTags.push(tag.name);
      }
    }
  }

  return {
    validTags,
    requiredCategories: {
      component: componentTags,
      featureGroup: featureGroupTags,
    },
  };
}
