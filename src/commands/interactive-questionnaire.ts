/**
 * Interactive Questionnaire for Foundation Discovery
 *
 * Guides AI through structured discovery workflow with specific question templates.
 * Supports both interactive mode and prefilled mode (from code analysis).
 */

export interface QuestionnaireOptions {
  mode: 'interactive' | 'from-discovery';
  discoveryData?: DiscoveryData;
}

export interface DiscoveryData {
  projectType?: string;
  personas?: string[];
  capabilities?: string[];
  problems?: string[];
}

export interface Question {
  id: string;
  section: string;
  text: string;
  helpText: string;
  exampleAnswer: string;
  required: boolean;
  prefillValue?: string;
  isPrefilled?: boolean;
}

export interface QuestionnaireState {
  currentQuestionIndex: number;
  totalQuestions: number;
  answers: Record<string, string>;
}

/**
 * Question templates for structured discovery
 */
export const QUESTION_SECTIONS = {
  vision: {
    name: 'Vision Questions',
    questions: [
      {
        id: 'core-purpose',
        section: 'vision',
        text: 'What is the core purpose?',
        helpText:
          'One sentence elevator pitch. Example: fspec helps AI agents follow ACDD workflow',
        exampleAnswer: 'fspec helps AI agents follow ACDD workflow',
        required: true,
      },
      {
        id: 'primary-users',
        section: 'vision',
        text: 'Who are the primary users?',
        helpText: 'List the main personas who will use this system',
        exampleAnswer: 'Developers using AI to write specifications',
        required: true,
      },
      {
        id: 'problem-solved',
        section: 'vision',
        text: 'What problem does this solve?',
        helpText: 'Focus on WHY this exists, not HOW it works',
        exampleAnswer:
          'Developers need structured way to write acceptance criteria',
        required: true,
      },
    ],
  },
  problemSpace: {
    name: 'Problem Space',
    questions: [
      {
        id: 'top-pain-points',
        section: 'problemSpace',
        text: 'What are the top 3-5 pain points?',
        helpText: 'List the most critical problems users face',
        exampleAnswer:
          'Manual specification writing, inconsistent formats, lack of validation',
        required: true,
      },
    ],
  },
  solutionSpace: {
    name: 'Solution Space',
    questions: [
      {
        id: 'key-capabilities',
        section: 'solutionSpace',
        text: 'What are the 3-7 key capabilities?',
        helpText: 'Focus on WHAT the system does, not HOW',
        exampleAnswer: 'Gherkin validation, tag management, coverage tracking',
        required: true,
      },
    ],
  },
};

/**
 * Build question list from templates
 */
export function buildQuestions(options: QuestionnaireOptions): Question[] {
  const questions: Question[] = [];

  for (const section of Object.values(QUESTION_SECTIONS)) {
    for (const template of section.questions) {
      const question: Question = { ...template };

      // Prefill if discovery data available
      if (options.mode === 'from-discovery' && options.discoveryData) {
        if (template.id === 'primary-users' && options.discoveryData.personas) {
          question.prefillValue = options.discoveryData.personas.join(', ');
          question.isPrefilled = true;
        }
      }

      questions.push(question);
    }
  }

  return questions;
}

/**
 * Format question display with help text and example
 */
export function formatQuestionDisplay(
  question: Question,
  state: QuestionnaireState
): string {
  const progress = `Question ${state.currentQuestionIndex + 1} of ${state.totalQuestions}`;
  let display = `${progress}\n\n`;
  display += `${question.text}\n`;
  display += `[HELP: ${question.helpText}]\n`;
  display += `[Example: ${question.exampleAnswer}]\n`;

  if (question.isPrefilled && question.prefillValue) {
    display += `\n[DETECTED] ${question.prefillValue}\n`;
    display += `Keep/Edit/Skip?\n`;
  }

  return display;
}

/**
 * Validate answer
 */
export function validateAnswer(
  answer: string,
  required: boolean
): {
  valid: boolean;
  error?: string;
} {
  if (required && (!answer || answer.trim() === '')) {
    return {
      valid: false,
      error: 'This question requires an answer. Please provide a response.',
    };
  }
  return { valid: true };
}

/**
 * Main questionnaire flow (returns structure, not full UI)
 */
export function runQuestionnaire(options: QuestionnaireOptions): {
  questions: Question[];
  formatQuestion: (index: number) => string;
  validate: (
    answer: string,
    questionId: string
  ) => {
    valid: boolean;
    error?: string;
  };
} {
  const questions = buildQuestions(options);
  const state: QuestionnaireState = {
    currentQuestionIndex: 0,
    totalQuestions: questions.length,
    answers: {},
  };

  return {
    questions,
    formatQuestion: (index: number) => {
      state.currentQuestionIndex = index;
      return formatQuestionDisplay(questions[index], state);
    },
    validate: (answer: string, questionId: string) => {
      const question = questions.find(q => q.id === questionId);
      if (!question) {
        return { valid: false, error: 'Question not found' };
      }
      return validateAnswer(answer, question.required);
    },
  };
}
