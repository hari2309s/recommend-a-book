import { Box, Card, DataList, Heading } from "@radix-ui/themes"

const RecommendationCard = () => {
  return <Box>
    <Card>
      <Heading size='6'>Book Title</Heading>
      <DataList.Root>
        <DataList.Item>
          <DataList.Label></DataList.Label>
        </DataList.Item>
      </DataList.Root>
    </Card>
  </Box>
}

export default RecommendationCard;
