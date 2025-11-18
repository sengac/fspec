/**
 * Aggregated fspec Bootstrap Documentation
 *
 * This file aggregates all bootstrap sections for testing purposes.
 * The actual bootstrap command dynamically combines these sections.
 */

import { getBootstrapFoundationSection } from '../slashCommandSections/bootstrapFoundation';
import { getBigPictureEventStormSection } from '../slashCommandSections/bigPictureEventStorm';
import { getEventStormSection } from '../slashCommandSections/eventStorm';
import { getCoverageTrackingSection } from '../slashCommandSections/coverageTracking';

export function getAggregatedBootstrapDocumentation(): string {
  return (
    getBootstrapFoundationSection() +
    '\n\n' +
    getBigPictureEventStormSection() +
    '\n\n' +
    getEventStormSection() +
    '\n\n' +
    getCoverageTrackingSection()
  );
}
