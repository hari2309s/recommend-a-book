import { supabase } from '../config/supabase';
import { SearchHistory } from '../types';

export class SearchHistoryService {
  async saveSearch(
    userId: string,
    query: string,
    recommendations: SearchHistory['recommendations']
  ): Promise<SearchHistory> {
    const { data, error } = await supabase
      .from('search_history')
      .insert({ user_id: userId, query, recommendations })
      .select()
      .single();

    if (error) {
      throw new Error(`Failed to save search: ${error.message}`);
    }

    return data;
  }

  async getSearchHistory(userId: string): Promise<SearchHistory[]> {
    const { data, error } = await supabase
      .from('search_history')
      .select('*')
      .eq('user_id', userId)
      .order('created_at', { ascending: false });

    if (error) {
      throw new Error(`Failed to fetch search history: ${error.message}`);
    }

    return data;
  }
}
