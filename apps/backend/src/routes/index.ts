import express, { Request, Response, Router } from 'express';
import { RecommendationController } from '@/controllers/recommendationController';

type AsyncRequestHandler = (req: Request, res: Response) => Promise<void>;

export const setupRoutes = (recommendationController: RecommendationController): Router => {
  const router = express.Router();

  const getRecommendationsHandler: AsyncRequestHandler = async (req, res) => {
    await recommendationController.getRecommendations(req, res);
  };

  const getSearchHistoryHandler: AsyncRequestHandler = async (req, res) => {
    await recommendationController.getSearchHistory(req, res);
  };

  router.post('/recommend', getRecommendationsHandler);
  router.get('/history', getSearchHistoryHandler);

  return router;
};
