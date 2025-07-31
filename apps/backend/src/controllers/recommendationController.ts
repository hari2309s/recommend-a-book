import { Request, Response } from 'express';
import { RecommendationService } from '@/services/recommendationService';
import { SearchHistoryService } from '@/services/searchHistoryService';
import { v4 as uuidv4 } from 'uuid';

export class RecommendationController {
  private recommendationService: RecommendationService;
  private searchHistoryService: SearchHistoryService;

  constructor(
    recommendationService: RecommendationService,
    searchHistoryService: SearchHistoryService
  ) {
    this.recommendationService = recommendationService;
    this.searchHistoryService = searchHistoryService;
  }

  getRecommendations = async (req: Request, res: Response) => {
    const { query, user_id, topK } = req.body;

    if (!query) {
      return res.status(400).json({ error: 'Query is required' });
    }

    try {
      const recommendations = await this.recommendationService.getRecommendations(
        query,
        topK ?? 10
      );

      const effectiveUserId = user_id || uuidv4();
      await this.searchHistoryService.saveSearch(effectiveUserId, query, recommendations);
      res.json({ recommendations, user_id: effectiveUserId });
    } catch (error) {
      console.error('Error fetching recommendations:', error);
      res.status(500).json({ error: 'Failed to fetch recommendations' });
    }
  };

  getSearchHistory = async (req: Request, res: Response) => {
    const { user_id } = req.query;

    if (!user_id) {
      return res.status(400).json({ error: 'user_id is required' });
    }

    try {
      const history = await this.searchHistoryService.getSearchHistory(user_id as string);
      res.json({ history });
    } catch (error) {
      console.error('Error fetching search history:', error);
      res.status(500).json({ error: 'Failed to fetch search history' });
    }
  };
}
